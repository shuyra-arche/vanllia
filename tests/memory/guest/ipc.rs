//! Two real Teams, retained writable translations, and a foreign holding transfer.
use super::support::*;
use core::{
    fmt::Write,
    sync::atomic::{AtomicU64, Ordering},
};
use kernel_api::{
    abi::memory::*,
    address::VirtAddr,
    boot::capsule::Capsule,
    model::{cell::Placement, flat::SlotPosition, identity::*, pend::*, unit::*},
};
use kernel_client::call::{room, unit};
fn word(base: usize, n: usize) -> &'static AtomicU64 {
    unsafe { &*((base + n * 8) as *const AtomicU64) }
}
fn wait(base: usize, n: usize, value: u64) {
    let end = room::clock() + 5_000_000_000;
    while word(base, n).load(Ordering::Acquire) != value {
        assert!(room::clock() < end, "IPC timeout at {n}, expected {value}");
        room::yield_();
    }
}
extern "C" fn child(at: usize, count: usize) -> ! {
    assert_eq!(count, 2);
    let args = unsafe { core::slice::from_raw_parts(at as *const usize, count) };
    let shared = args[0];
    let parent = TaskId::new(args[1]);
    for sequence in 1..=128 {
        wait(shared, 0, sequence);
        assert_eq!(word(shared, 2).load(Ordering::Relaxed), sequence * 7);
        word(shared, 3).store(sequence * 11, Ordering::Relaxed);
        word(shared, 1).store(sequence, Ordering::Release);
    }
    let slot = create(1);
    mutate(Action::Occupy {
        slot,
        expected: slot_status(slot).version,
    });
    let (memo, va) = reserve(space());
    insert(memo, slot);
    publish(memo);
    unsafe { (va.get() as *mut u64).write_volatile(0xbeef1234) };
    retract(memo);
    let token = accord(slot.token(), parent, pend_info(slot.token()).limits);
    word(shared, 4).store(token.get() as u64, Ordering::Relaxed);
    word(shared, 5).store(slot_status(slot).page, Ordering::Relaxed);
    word(shared, 6).store(1, Ordering::Release);
    wait(shared, 7, 1);
    assert_eq!(
        slot_status(slot).phase,
        kernel_api::model::flat::SlotPhase::Missing
    );
    unit::exit(73, 0, VirtAddr::new(0), 0, ExitScope::Task)
}
pub fn run() {
    let descriptor = UnitCreate::Team {
        kind: ProgramKind::User,
        guard: ControlId::NONE,
        nest: None,
        member_exit: true,
    }
    .to_bytes();
    let CreatedUnit::Team { team, nest } =
        unit::create(VirtAddr::new(descriptor.as_ptr() as usize)).unwrap()
    else {
        panic!("Team")
    };
    let source = image();
    let capsule = Capsule::parse(source).unwrap();
    // Load through ordinary Flat/Nest authority. Every writable staging mapping
    // is retired before executable publication and its required FENCE.I.
    for i in 0..capsule.count {
        let region = capsule.region(i).unwrap();
        for page in 0..region.pages {
            let slot = create(1);
            mutate(Action::Occupy {
                slot,
                expected: slot_status(slot).version,
            });
            if page < region.data_pages {
                let (staging, va) = reserve(space());
                insert(staging, slot);
                publish(staging);
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        source.as_ptr().add(region.payload + page * 4096),
                        va.get() as *mut u8,
                        4096,
                    )
                };
                retract(staging);
            }
            let (memo, _) = reserve_at(
                nest,
                Placement::exact(VirtAddr::new(region.va + page * 4096), 4096).unwrap(),
                region.flags,
            );
            insert(memo, slot);
            publish(memo);
        }
    }
    let slot = create(1);
    mutate(Action::Occupy {
        slot,
        expected: slot_status(slot).version,
    });
    let (local, local_va) = reserve(space());
    insert(local, slot);
    publish(local);
    let (remote, remote_va) = reserve(nest);
    insert(remote, slot);
    publish(remote);
    assert_eq!(slot_status(slot).obligations, 2);
    let UnitTarget::Task(parent) = task_info(UnitQuery::Caller).target else {
        panic!("caller")
    };
    let args = [remote_va.get(), parent.get()];
    let descriptor = UnitCreate::Task {
        team: Some(team),
        entry: VirtAddr::new(child as *const () as usize),
        args: VirtAddr::new(args.as_ptr() as usize),
        count: 2,
        stack: VirtAddr::new(0),
    }
    .to_bytes();
    let CreatedUnit::Task(task) =
        unit::create(VirtAddr::new(descriptor.as_ptr() as usize)).unwrap()
    else {
        panic!("Task")
    };
    accord(license(), task, pend_info(license()).limits);
    accord(nest, task, pend_info(nest).limits);
    unit::resume(UnitTarget::Task(task)).unwrap();
    let base = local_va.get();
    let generation = memo_status(local).generation;
    let began = room::clock();
    for sequence in 1..=64 {
        exchange(base, sequence);
    }
    let retained_ns = room::clock() - began;
    assert_eq!(
        memo_status(local).generation,
        generation,
        "long-lived IPC must not republish"
    );
    let began = room::clock();
    for sequence in 65..=128 {
        retract(local);
        publish(local);
        exchange(base, sequence);
    }
    let remap_ns = room::clock() - began;
    assert_eq!(slot_status(slot).obligations, 2);
    wait(base, 6, 1);
    let token = PendId::mint(word(base, 4).load(Ordering::Relaxed) as usize);
    let Target::Flat(flat) = pend_info(token).target else {
        panic!("foreign Flat")
    };
    let foreign = SlotPosition::new(token, flat, 0).unwrap();
    let destination = create(1);
    let page = word(base, 5).load(Ordering::Relaxed);
    mutate(Action::Pull {
        local: destination,
        local_version: slot_status(destination).version,
        remote: foreign,
        remote_version: slot_status(foreign).version,
        maintenance: None,
    });
    assert_eq!(
        slot_status(destination).page,
        page,
        "movement preserves Page identity"
    );
    let (received, va) = reserve(space());
    insert(received, destination);
    publish(received);
    assert_eq!(
        unsafe { (va.get() as *const u64).read_volatile() },
        0xbeef1234
    );
    word(base, 7).store(1, Ordering::Release);
    assert_eq!(join(task).result, Some(73));
    retract(remote);
    assert_eq!(slot_status(slot).obligations, 1);
    word(base, 0).store(999, Ordering::Release);
    assert_eq!(
        word(base, 0).load(Ordering::Acquire),
        999,
        "unrelated writable Memo remains usable"
    );
    let _ = writeln!(
        Console,
        "PASS native-ipc teams=2 messages=128 independent-retract=1 foreign-transfer=1 page-identity=1"
    );
    let _ = writeln!(
        Console,
        "PERF native-ipc messages=64 retained_ns={retained_ns} remap_ns={remap_ns} retained_publish=0 retained_retract=0 baseline_publish=64 baseline_retract=64"
    );
}
fn exchange(base: usize, sequence: u64) {
    word(base, 2).store(sequence * 7, Ordering::Relaxed);
    word(base, 0).store(sequence, Ordering::Release);
    wait(base, 1, sequence);
    assert_eq!(word(base, 3).load(Ordering::Relaxed), sequence * 11);
}
