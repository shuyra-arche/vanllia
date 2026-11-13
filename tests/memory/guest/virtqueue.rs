//! Real split Virtqueue: CPU and PCI requester retain the same writable RAM.
use super::support::*;
use core::fmt::Write;
use kernel_api::model::cell::Placement;
use kernel_api::{
    abi::memory::*,
    model::{nest::*, pend::*},
};
use kernel_client::{call::room, memory::Response};

const BAR: usize = 0x4000_0000;
const IO: usize = 0x1000_0000;
fn order() {
    unsafe { core::arch::asm!("fence iorw, iorw", options(nostack)) }
}
fn r8(at: usize) -> u8 {
    order();
    let v = unsafe { (at as *const u8).read_volatile() };
    order();
    v
}
fn r16(at: usize) -> u16 {
    order();
    let v = unsafe { (at as *const u16).read_volatile() };
    order();
    v
}
fn r32(at: usize) -> u32 {
    order();
    let v = unsafe { (at as *const u32).read_volatile() };
    order();
    v
}
fn w8(at: usize, v: u8) {
    order();
    unsafe { (at as *mut u8).write_volatile(v) };
    order()
}
fn w16(at: usize, v: u16) {
    order();
    unsafe { (at as *mut u16).write_volatile(v) };
    order()
}
fn w32(at: usize, v: u32) {
    order();
    unsafe { (at as *mut u32).write_volatile(v) };
    order()
}
fn w64(at: usize, v: u64) {
    unsafe { (at as *mut u64).write_volatile(v) }
}
fn reset() {
    w8(BAR + 20, 0);
    assert_eq!(r8(BAR + 20), 0);
    assert_eq!(r16(BAR + 28), 0)
}
fn start() {
    reset();
    w8(BAR + 20, 1);
    w8(BAR + 20, 3);
    w32(BAR, 1);
    assert_eq!(
        r32(BAR + 4) & 3,
        3,
        "VERSION_1 and ACCESS_PLATFORM required"
    );
    w32(BAR + 8, 0);
    w32(BAR + 12, 0);
    w32(BAR + 8, 1);
    w32(BAR + 12, 3);
    w8(BAR + 20, 11);
    assert_ne!(r8(BAR + 20) & 8, 0);
    w16(BAR + 22, 0);
    assert!(r16(BAR + 24) >= 8);
    w16(BAR + 24, 8);
    w16(BAR + 26, 0xffff);
    w32(BAR + 32, IO as u32);
    w32(BAR + 36, 0);
    w32(BAR + 40, (IO + 0x100) as u32);
    w32(BAR + 44, 0);
    w32(BAR + 48, (IO + 0x200) as u32);
    w32(BAR + 52, 0);
    assert_eq!(r16(BAR + 30), 0);
    w16(BAR + 28, 1);
    w8(BAR + 20, 15);
}
fn descriptor(base: usize, index: usize, offset: usize, len: u32, flags: u16, next: u16) {
    let at = base + 16 * index;
    w64(at, (IO + offset) as u64);
    w32(at + 8, len);
    w16(at + 12, flags);
    w16(at + 14, next);
}
#[derive(Clone, Copy)]
#[repr(u32)]
enum Operation {
    Read = 0,
    Write = 1,
}
fn submit(base: usize, sequence: u16, operation: Operation, pattern: u8) {
    w32(base + 0x300, operation as u32);
    w32(base + 0x304, 0);
    w64(base + 0x308, 0);
    for i in 0..512 {
        w8(
            base + 0x400 + i,
            if matches!(operation, Operation::Write) {
                pattern.wrapping_add(i as u8)
            } else {
                0
            },
        )
    }
    w8(base + 0x600, 0xff);
    descriptor(base, 0, 0x300, 16, 1, 1);
    descriptor(
        base,
        1,
        0x400,
        512,
        if matches!(operation, Operation::Write) {
            1
        } else {
            3
        },
        2,
    );
    descriptor(base, 2, 0x600, 1, 2, 0);
    w16(base + 0x104 + 2 * ((sequence as usize - 1) % 8), 0);
    order();
    w16(base + 0x102, sequence);
    w16(BAR + 0x3000, 0);
    let end = room::clock() + 2_000_000_000;
    while r16(base + 0x202) != sequence {
        assert!(room::clock() < end, "Virtqueue completion timed out");
        room::yield_();
    }
    order();
    assert_eq!(r32(base + 0x204 + 8 * ((sequence as usize - 1) % 8)), 0);
    assert_eq!(r8(base + 0x600), 0);
    if matches!(operation, Operation::Read) {
        for i in 0..512 {
            assert_eq!(r8(base + 0x400 + i), pattern.wrapping_add(i as u8));
        }
    }
}
pub fn run() {
    let external = find(|p| {
        if p.limits.scope != PendScope::Nest(NestScope::Whole) {
            return false;
        }
        let Target::Nest(nest) = p.target else {
            return false;
        };
        let Snapshot::Nest(s) = inspect(Query::Nest {
            token: p.token,
            nest,
        }) else {
            return false;
        };
        s.info.kind == NestKind::External
    })
    .token;
    let slot = create(1);
    mutate(Action::Occupy {
        slot,
        expected: slot_status(slot).version,
    });
    let (cpu, address) = reserve(space());
    insert(cpu, slot);
    publish(cpu);
    let (device, io) = reserve_at(
        external,
        Placement::exact(kernel_api::address::VirtAddr::new(IO), 4096).unwrap(),
        6,
    );
    assert_eq!(io.get(), IO);
    insert(device, slot);
    let publication = request(Action::Publish {
        memo: device,
        expected: memo_status(device).generation,
        maintenance: None,
    });
    let bytes = publication.to_bytes();
    let frame = Frame::new(Call::from_request(publication), bytes.as_ptr() as usize, 0).unwrap();
    let result =
        unsafe { kernel_client::transport::trap(frame.number() as usize, frame.registers()) };
    assert_eq!(
        Delivery::from_registers([result.0 as u64, result.1 as u64, result.2 as u64]),
        Some(Delivery::Failed(Failure::ReplyFault))
    );
    let deadline = room::clock() + 3_000_000_000;
    while memo_status(device).phase != MemoPhase::Published {
        assert!(room::clock() < deadline);
        room::yield_();
    }
    let Response::Reply(completed) = call(Action::InspectResult {
        domain: Domain::Nest,
        request: publication.id().unwrap(),
    }) else {
        panic!("result")
    };
    assert!(matches!(completed.outcome(), Outcome::Committed(_)));
    assert_eq!(slot_status(slot).obligations, 2);
    let cg = memo_status(cpu).generation;
    let dg = memo_status(device).generation;
    let base = address.get();
    start();
    let before = room::clock();
    for n in 0..32 {
        submit(base, 2 * n + 1, Operation::Write, n as u8);
        submit(base, 2 * n + 2, Operation::Read, n as u8);
    }
    let elapsed = room::clock() - before;
    assert_eq!(memo_status(cpu).generation, cg);
    assert_eq!(memo_status(device).generation, dg);
    // Used completions do not retire device access. The trusted proof must
    // retain the Page while the queue remains enabled, even with no request.
    let Response::Reply(reply) = call(Action::Retract {
        memo: device,
        expected: dg,
    }) else {
        panic!("Retract")
    };
    assert!(
        matches!(reply.outcome(), Outcome::Pending(_)),
        "{:?}",
        reply.outcome()
    );
    assert_eq!(slot_status(slot).obligations, 2);
    reset();
    let deadline = room::clock() + 3_000_000_000;
    while memo_status(device).phase != MemoPhase::Associated {
        assert!(room::clock() < deadline);
        room::yield_();
    }
    let Response::Reply(completed) = call(Action::InspectResult {
        domain: Domain::Nest,
        request: reply.request(),
    }) else {
        panic!("retirement result")
    };
    assert!(matches!(completed.outcome(), Outcome::Committed(_)));
    let retry = nest::Call::new(
        Some(reply.request()),
        Action::Retract {
            memo: device,
            expected: dg,
        },
    )
    .unwrap();
    let Response::Reply(replayed) = kernel_client::memory::nest::invoke(retry).unwrap() else {
        panic!("replay")
    };
    assert_eq!(completed, replayed);
    assert_eq!(slot_status(slot).obligations, 1);
    w32(base + 0x700, 0xface1234);
    assert_eq!(r32(base + 0x700), 0xface1234);
    retract(cpu);
    let maintenance = block(slot);
    mutate(Action::Vacate {
        slot,
        expected: slot_status(slot).version,
        maintenance,
    });
    mutate(Action::Allow { maintenance });
    let _ = writeln!(
        Console,
        "PASS native-virtqueue requests=64 verified-read-bytes=16384 cpu-external-rw=1 retained-publications=1 used-not-retirement=1 reset-drain=1 independent-retract=1 reply-loss-recovered=1 exact-replay=1"
    );
    let _ = writeln!(
        Console,
        "PERF native-virtqueue requests=64 elapsed_ns={elapsed} publish_during_io=0 retract_during_io=0"
    );
    release(device);
    lifecycle(external);
}

fn release(memo: MemoPosition) {
    let Snapshot::Reservation(s) = inspect(Query::Reservation {
        token: memo.token(),
        nest: memo.nest(),
        reservation: memo.reservation(),
    }) else {
        panic!("Reservation")
    };
    mutate(Action::Release {
        token: memo.token(),
        nest: memo.nest(),
        reservation: memo.reservation(),
        expected: s.version,
    });
}
#[derive(Clone, Copy, Debug)]
enum Close {
    Revoke,
    Return,
    Release,
    Delete,
}
fn lifecycle(external: kernel_api::model::identity::PendId) {
    use kernel_api::{
        address::VirtAddr,
        model::{flat::*, identity::Mark, unit::*},
    };
    use kernel_client::call::pend;
    for mode in [Close::Revoke, Close::Return, Close::Release, Close::Delete] {
        let slot = create(1);
        mutate(Action::Occupy {
            slot,
            expected: slot_status(slot).version,
        });
        let (cpu, address) = reserve(space());
        insert(cpu, slot);
        publish(cpu);
        let (device, _) = reserve_at(
            external,
            Placement::exact(VirtAddr::new(IO), 4096).unwrap(),
            6,
        );
        insert(device, slot);
        let bytes = AccordRequest {
            src: device.token(),
            target: AccordTarget::Caller,
            limits: pend_info(device.token()).limits,
            mark: Mark::NONE,
        }
        .to_bytes();
        let branch = pend::accord(VirtAddr::new(bytes.as_ptr() as usize)).unwrap();
        let local = MemoPosition::new(branch, device.nest(), device.reservation(), 0).unwrap();
        let Response::Reply(r) = call(Action::Publish {
            memo: local,
            expected: memo_status(device).generation,
            maintenance: None,
        }) else {
            panic!("Publish")
        };
        assert!(matches!(
            r.outcome(),
            Outcome::Pending(_) | Outcome::Committed(_)
        ));
        let deadline = room::clock() + 3_000_000_000;
        while memo_status(device).phase != MemoPhase::Published {
            assert!(room::clock() < deadline);
            room::yield_();
        }
        start();
        submit(address.get(), 1, Operation::Write, 77);
        match mode {
            Close::Revoke => {
                let UnitTarget::Task(task) = task_info(UnitQuery::Caller).target else {
                    panic!("Task")
                };
                let _ = pend::revoke(task, branch);
            }
            Close::Return => {
                let _ = pend::return_(branch);
            }
            Close::Release => {
                let Snapshot::Reservation(s) = inspect(Query::Reservation {
                    token: device.token(),
                    nest: device.nest(),
                    reservation: device.reservation(),
                }) else {
                    panic!("Reservation")
                };
                let Response::Reply(r) = call(Action::Release {
                    token: device.token(),
                    nest: device.nest(),
                    reservation: device.reservation(),
                    expected: s.version,
                }) else {
                    panic!("Release")
                };
                assert!(
                    matches!(
                        r.outcome(),
                        Outcome::Pending(_)
                            | Outcome::Failed {
                                failure: Failure::Busy,
                                retained: Some(_)
                            }
                    ),
                    "{mode:?}: {:?}",
                    r.outcome()
                );
            }
            Close::Delete => {
                let Snapshot::Nest(s) = inspect(Query::Nest {
                    token: external,
                    nest: device.nest(),
                }) else {
                    panic!("Nest")
                };
                let Response::Reply(r) = call(Action::DeleteNest {
                    token: external,
                    nest: device.nest(),
                    expected: s.version,
                }) else {
                    panic!("Delete")
                };
                assert!(
                    matches!(
                        r.outcome(),
                        Outcome::Pending(_)
                            | Outcome::Failed {
                                failure: Failure::Busy,
                                retained: Some(_)
                            }
                    ),
                    "{mode:?}: {:?}",
                    r.outcome()
                );
            }
        }
        assert_eq!(
            slot_status(slot).obligations,
            2,
            "{mode:?}: authority loss must retain CPU and DMA responsibility"
        );
        w32(address.get() + 0x700, 0x1234);
        assert_eq!(r32(address.get() + 0x700), 0x1234);
        // Blocking retires CPU access but cannot release the still-enabled DMA.
        let Response::Reply(r) = call(Action::Block {
            start: slot,
            count: 1,
            recover: None,
        }) else {
            panic!("Block")
        };
        let (Outcome::Pending(Value::Maintenance { block, .. })
        | Outcome::Failed {
            failure: Failure::Busy,
            retained: Some(Value::Maintenance { block, .. }),
        }) = r.outcome()
        else {
            panic!("{mode:?}: {:?}", r.outcome())
        };
        let maintenance = Maintenance {
            token: slot.token(),
            block,
        };
        assert_eq!(slot_status(slot).obligations, 1);
        let Response::Reply(r) = call(Action::Vacate {
            slot,
            expected: slot_status(slot).version,
            maintenance,
        }) else {
            panic!("Vacate")
        };
        assert!(matches!(r.outcome(), Outcome::Rejected(Failure::Busy)));
        reset();
        let deadline = room::clock() + 3_000_000_000;
        loop {
            let Response::Reply(r) = call(Action::Block {
                start: slot,
                count: 1,
                recover: Some(block),
            }) else {
                panic!("Block")
            };
            if matches!(
                r.outcome(),
                Outcome::Committed(Value::Maintenance {
                    phase: BlockPhase::Held,
                    ..
                })
            ) {
                break;
            }
            assert!(room::clock() < deadline, "{mode:?}: drain stalled");
            room::yield_();
        }
        assert_eq!(slot_status(slot).obligations, 0);
        mutate(Action::Vacate {
            slot,
            expected: slot_status(slot).version,
            maintenance,
        });
        mutate(Action::Allow { maintenance });
        if !matches!(mode, Close::Delete) {
            release(device);
        } else {
            let Snapshot::Nest(s) = inspect(Query::Nest {
                token: external,
                nest: device.nest(),
            }) else {
                panic!("Nest")
            };
            let deadline = room::clock() + 3_000_000_000;
            loop {
                let Response::Reply(r) = call(Action::DeleteNest {
                    token: external,
                    nest: device.nest(),
                    expected: s.version,
                }) else {
                    panic!("Delete")
                };
                if matches!(r.outcome(), Outcome::Committed(_)) {
                    break;
                }
                assert!(
                    matches!(
                        r.outcome(),
                        Outcome::Failed {
                            failure: Failure::Busy,
                            ..
                        }
                    ),
                    "{:?}",
                    r.outcome()
                );
                assert!(
                    room::clock() < deadline,
                    "Delete did not settle after reset"
                );
                room::yield_();
            }
        }
        let _ = writeln!(
            Console,
            "PASS acceptance-lifecycle mode={mode:?} authority-closed=1 cpu-independent=1 dma-retained=1 maintenance-pending=1 reset-settled=1"
        );
    }
}
