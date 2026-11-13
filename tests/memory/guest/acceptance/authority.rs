use super::worker::Worker;
use super::*;
use core::fmt::Write;
use kernel_api::model::{cell::Placement, flat::SlotPosition};
use kernel_client::call::{sign, unit};

pub fn run() {
    let slot = create(1);
    let (memo, _) = reserve(space());
    insert(memo, slot);
    let worker = Worker::new();
    let flat = accord(
        slot.token(),
        worker.0,
        Limits {
            operations: Operations::Flat(FlatRights::INSPECT),
            ..pend_info(slot.token()).limits
        },
    );
    let inspected = SlotPosition::new(flat, slot.flat(), 0).unwrap();
    assert!(matches!(
        worker.call(Action::Inspect(Query::Slot(inspected))),
        Ok(Response::Observation(_))
    ));
    rejected(worker.call(Action::Occupy {
        slot: inspected,
        expected: slot_status(slot).version,
    }));
    let token = accord(
        memo.token(),
        worker.0,
        Limits {
            operations: Operations::Nest(NestRights::FAULT),
            ..pend_info(memo.token()).limits
        },
    );
    let local = at(memo, token, 0);
    rejected(worker.call(Action::Publish {
        memo: local,
        expected: memo_status(memo).generation,
        maintenance: None,
    }));
    rejected(worker.call(Action::Insert {
        target: local,
        expected: memo_status(memo).generation,
        source: Source::Flat {
            position: inspected,
            expected: slot_status(slot).version,
        },
    }));
    let signal = sign::create(license()).unwrap();
    let child_sign = accord(signal, worker.0, pend_info(signal).limits);
    let response = worker
        .call(Action::FaultRoute {
            token,
            nest: memo.nest(),
            reservation: memo.reservation(),
            expected: reservation(memo).version,
            epoch: FaultEpoch::new(1).unwrap(),
            route: Some(FaultRoute {
                sign: child_sign,
                timeout_ns: 1_000_000_000,
            }),
        })
        .unwrap();
    let Response::Reply(r) = response else {
        panic!("route")
    };
    let Outcome::Committed(Value::FaultRoute { epoch: old, .. }) = r.outcome() else {
        panic!("{:?}", r.outcome())
    };
    let replacement = crate::fault::configure(
        memo,
        Some(FaultRoute {
            sign: signal,
            timeout_ns: 1_000_000_000,
        }),
    );
    assert_ne!(old, replacement);
    rejected(worker.call(Action::Inspect(Query::FaultEvent {
        token,
        nest: memo.nest(),
        reservation: memo.reservation(),
        epoch: old,
        after: 0,
        wait: kernel_api::model::wait::Wait::Poll,
    })));
    let stale = memo_status(memo).generation;
    insert(memo, create(1));
    rejected(try_call(Action::Publish {
        memo,
        expected: stale,
        maintenance: None,
    }));
    crate::fault::configure(memo, None);
    worker.stop();
    sign::delete(signal).unwrap();
    foreign_control();
    let _ = writeln!(
        Console,
        "PASS acceptance-authority inspect-only=reject-data fault-only=route-not-data old-route=reject old-memo=reject foreign-control=not-local-address"
    );
}

extern "C" fn foreign(at: usize, count: usize) -> ! {
    assert_eq!(count, 1);
    let address = unsafe { *(at as *const usize) };
    let p = find(|p| {
        matches!(
            p.limits.scope,
            PendScope::Nest(NestScope::Reservation { .. })
        )
    });
    let Target::Nest(nest) = p.target else {
        panic!("Nest")
    };
    let PendScope::Nest(NestScope::Reservation { id, .. }) = p.limits.scope else {
        panic!("Reservation")
    };
    let memo = MemoPosition::new(p.token, nest, id, 0).unwrap();
    publish(memo);
    let _ = writeln!(
        Console,
        "PASS acceptance-foreign-control published-parent=1"
    );
    // This address exists only in the parent's Nest. A control grant cannot
    // create a translation in this Team, even after a successful Publish.
    unsafe {
        (address as *const u64).read_volatile();
    }
    panic!("foreign control unexpectedly granted a local translation");
}
fn foreign_control() {
    let (team, _) = crate::ipc::create_team();
    let slot = create(1);
    mutate(Action::Occupy {
        slot,
        expected: slot_status(slot).version,
    });
    let (memo, address) = reserve_at(
        space(),
        Placement::exact(VirtAddr::new(0x60000000), 4096).unwrap(),
        6,
    );
    insert(memo, slot);
    let args = [address.get()];
    let bytes = UnitCreate::Task {
        team: Some(team),
        entry: VirtAddr::new(foreign as *const () as usize),
        args: VirtAddr::new(args.as_ptr() as usize),
        count: 1,
        stack: VirtAddr::new(0),
    }
    .to_bytes();
    let CreatedUnit::Task(task) = unit::create(VirtAddr::new(bytes.as_ptr() as usize)).unwrap()
    else {
        panic!("Task")
    };
    accord(memo.token(), task, pend_info(memo.token()).limits);
    unit::resume(UnitTarget::Task(task)).unwrap();
    assert!(matches!(join(task).exit, Some((ExitCause::Fault, _))));
    assert_eq!(slot_status(slot).obligations, 0);
}
