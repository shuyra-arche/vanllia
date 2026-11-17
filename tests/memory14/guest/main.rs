#![no_std]
#![no_main]

use core::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering},
};
use kernel_api::{
    abi::memory::{self, Outcome, RequestId, Snapshot, Source, Value, flat, nest},
    address::{PageCount, VirtAddr, PAGE_SIZE},
    model::{
        cell::{CellLevel, Placement},
        flat::{SlotPosition, SlotPhase},
        identity::PendId,
        nest::{CreateSpanRequest, MemoPosition, NestKind, MemoPhase},
        pend::{NestScope, PendInfo, PendScope, Target},
        unit::ExitScope,
    },
};
use kernel_client::{
    call::{debug, pend, room, unit},
    memory::Response,
};

core::arch::global_asm!(
    ".section .text.entry",
    ".global _start",
    "_start:",
    "tail run"
);

struct Console;
impl Write for Console {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for bytes in text.as_bytes().chunks(128) {
            let mut offset = 0;
            while offset < bytes.len() {
                let count = debug::put(
                    VirtAddr::new(bytes[offset..].as_ptr() as usize),
                    bytes.len() - offset,
                )
                .map_err(|_| core::fmt::Error)?;
                if count == 0 {
                    return Err(core::fmt::Error);
                }
                offset += count;
            }
        }
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(Console, "FAIL native-memory14 {info}");
    unit::exit(1, 0, VirtAddr::new(0), 0, ExitScope::Task)
}

static NEXT: AtomicU64 = AtomicU64::new(1);
fn number() -> RequestId {
    RequestId::new(NEXT.fetch_add(1, Ordering::Relaxed)).unwrap()
}

fn flat_mutate(action: flat::Action) -> Value {
    let id = number();
    let call = flat::Call::new(Some(id), action).expect("valid Flat action");
    let Response::Reply(mut reply) = kernel_client::memory::flat::invoke(call)
        .expect("Flat delivery")
    else {
        panic!("Flat mutation returned an observation")
    };
    let deadline = room::clock() + 8_000_000_000;
    loop {
        match reply.outcome() {
            Outcome::Committed(value) => return value,
            Outcome::Pending(_) => {
                assert!(room::clock() < deadline, "Flat result did not settle");
                room::yield_();
                let query = flat::Call::new(None, flat::Action::InspectResult { request: id })
                    .unwrap();
                let Response::Reply(next) = kernel_client::memory::flat::invoke(query)
                    .expect("Flat retained delivery")
                else {
                    panic!("Flat result inspection returned an observation")
                };
                reply = next;
            }
            result => panic!("Flat mutation {action:?}: {result:?}"),
        }
    }
}

fn nest_mutate(action: nest::Action) -> Value {
    let id = number();
    let call = nest::Call::new(Some(id), action).expect("valid Nest action");
    let Response::Reply(mut reply) = kernel_client::memory::nest::invoke(call)
        .expect("Nest delivery")
    else {
        panic!("Nest mutation returned an observation")
    };
    let deadline = room::clock() + 8_000_000_000;
    loop {
        match reply.outcome() {
            Outcome::Committed(value) => return value,
            Outcome::Pending(_) => {
                assert!(room::clock() < deadline, "Nest result did not settle");
                room::yield_();
                let query = nest::Call::new(None, nest::Action::InspectResult { request: id })
                    .unwrap();
                let Response::Reply(next) = kernel_client::memory::nest::invoke(query)
                    .expect("Nest retained delivery")
                else {
                    panic!("Nest result inspection returned an observation")
                };
                reply = next;
            }
            result => panic!("Nest mutation {action:?}: {result:?}"),
        }
    }
}

fn flat_inspect(query: flat::Query) -> Snapshot {
    let call = flat::Call::new(None, flat::Action::Inspect(query)).unwrap();
    let Response::Observation(observation) = kernel_client::memory::flat::invoke(call)
        .expect("Flat observation delivery")
    else {
        panic!("Flat inspect returned a mutation")
    };
    observation.snapshot()
}

fn nest_inspect(query: nest::Query) -> Snapshot {
    let call = nest::Call::new(None, nest::Action::Inspect(query)).unwrap();
    let Response::Observation(observation) = kernel_client::memory::nest::invoke(call)
        .expect("Nest observation delivery")
    else {
        panic!("Nest inspect returned a mutation")
    };
    observation.snapshot()
}

fn info(token: PendId) -> PendInfo {
    let mut words = [0; PendInfo::WORDS];
    pend::inspect(token, VirtAddr::new(words.as_mut_ptr() as usize)).unwrap();
    PendInfo::from_words(words).unwrap()
}

fn find(select: impl Fn(PendInfo) -> bool) -> PendId {
    let mut after = PendId::NONE;
    loop {
        let mut words = [0; PendInfo::WORDS];
        assert_eq!(
            pend::collect(after, VirtAddr::new(words.as_mut_ptr() as usize), 1).unwrap(),
            1
        );
        let entry = PendInfo::from_words(words).unwrap();
        if select(entry) {
            return entry.token;
        }
        after = entry.token;
    }
}

fn license() -> PendId {
    find(|entry| entry.target == Target::None)
}

fn space() -> (PendId, kernel_api::model::nest::NestId) {
    let token = find(|entry| {
        if entry.limits.scope != PendScope::Nest(NestScope::Whole) {
            return false;
        }
        let Target::Nest(nest) = entry.target else {
            return false;
        };
        matches!(
            nest_inspect(nest::Query::Nest { token: entry.token, nest }),
            Snapshot::Nest(status) if status.info.kind == NestKind::Internal
        )
    });
    let Target::Nest(nest) = info(token).target else {
        panic!("Internal Nest is missing")
    };
    (token, nest)
}

#[unsafe(no_mangle)]
extern "C" fn run(at: usize, count: usize) -> ! {
    use kernel_api::boot::{args, ledger::Entries};
    assert_eq!(kernel_api::abi::VERSION, 14);
    assert_eq!(count, args::LEN);
    let args = unsafe { core::slice::from_raw_parts(at as *const usize, count) };
    let ledger = unsafe {
        core::slice::from_raw_parts(args[args::LEDGER] as *const u8, args[args::LEDGER_LEN])
    };
    assert!(Entries::new(ledger).is_some());
    let _ = writeln!(Console, "START native-memory14 abi=14 ledger=14");

    let license = license();
    let (space, nest) = space();
    let Value::CreatedFlat { token, flat, .. } = flat_mutate(flat::Action::Create {
        license,
        count: PageCount::new(1).unwrap(),
        level: CellLevel::BASE,
    }) else {
        panic!("Flat creation result")
    };
    let slot = SlotPosition::new(token, flat, 0).unwrap();
    let Snapshot::Slot(initial) = flat_inspect(flat::Query::Slot(slot)) else {
        panic!("Flat slot status")
    };
    assert_eq!(initial.phase, SlotPhase::Fresh);
    flat_mutate(flat::Action::Occupy {
        slot,
        expected: initial.version,
    });

    let Snapshot::Nest(current) = nest_inspect(nest::Query::Nest { token: space, nest }) else {
        panic!("Nest status")
    };
    let Value::Span { range, address, .. } = nest_mutate(nest::Action::Reserve {
        token: space,
        nest,
        expected: current.version,
        request: CreateSpanRequest {
            placement: Placement::lowest(PAGE_SIZE).unwrap(),
            count: PageCount::new(1).unwrap(),
            level: CellLevel::BASE,
            access: 6,
        },
    }) else {
        panic!("Nest Reserve did not return a range Pend")
    };
    let memo = MemoPosition::new(range, 0).unwrap();
    let Snapshot::Memo(empty) = nest_inspect(nest::Query::Memo(memo)) else {
        panic!("Memo status")
    };
    nest_mutate(nest::Action::Insert {
        target: memo,
        expected: empty.generation,
        source: Source::Flat {
            position: slot,
            expected: match flat_inspect(flat::Query::Slot(slot)) {
                Snapshot::Slot(status) => status.version,
                _ => panic!("Slot status after Occupy"),
            },
        },
    });
    let Snapshot::Memo(associated) = nest_inspect(nest::Query::Memo(memo)) else {
        panic!("Associated Memo status")
    };
    assert_eq!(associated.phase, MemoPhase::Associated);
    nest_mutate(nest::Action::Publish {
        memo,
        expected: associated.generation,
        maintenance: None,
    });
    unsafe { (address.get() as *mut u64).write_volatile(0x14_ab1_14_ab1) };
    assert_eq!(
        unsafe { (address.get() as *const u64).read_volatile() },
        0x14_ab1_14_ab1
    );
    let _ = writeln!(
        Console,
        "PASS memory-abi14 flat=created occupied=1 nest=reserved memo=published rw=1"
    );
    unit::exit(0, 0, VirtAddr::new(0), 0, ExitScope::Task)
}
