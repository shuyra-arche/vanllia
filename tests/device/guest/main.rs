#![no_std]
#![no_main]
use core::fmt::{self, Write};
use kernel_api::model::{
    identity::{TeamId, VirtAddr},
    memory::{
        AcquireSource, Allocation, FlatReference, MappingControl, MappingId, MappingSelection,
        Placement, ReferenceRange,
    },
    unit::{ExitCause, JoinReply, ProgramKind, UnitTarget},
    wait::Wait,
};
use kernel_client::call::{chrono, debug, device, memory, pend, room, share_view, sign, unit};

core::arch::global_asm!(
    ".section .text.entry",
    ".global _start",
    "_start:",
    "tail manager"
);
unsafe extern "C" {
    static __image_start: u8;
    static __image_end: u8;
}
struct Serial;
impl Write for Serial {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let _ = debug::put(VirtAddr::new(text.as_ptr() as usize), text.len());
        Ok(())
    }
}
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(Serial, "FAIL device lifecycle: {info}");
    room::reap(99, VirtAddr::new(0), 0)
}
fn allocation(bytes: usize) -> FlatReference {
    memory::acquire(
        AcquireSource::Allocation(Allocation::new(bytes, 4096).unwrap()),
        kernel_api::model::unit::ControlId::NONE,
    )
    .unwrap()
}
fn attach(
    reference: FlatReference,
    count: usize,
    team: TeamId,
    placement: Placement,
    access: u64,
) -> Option<(MappingId, VirtAddr)> {
    let range = ReferenceRange::new(reference, 0, count).unwrap().encode();
    let place = placement.encode();
    memory::attach(
        VirtAddr::new(range.as_ptr() as usize),
        team,
        VirtAddr::new(place.as_ptr() as usize),
        access,
        MappingControl::Caller,
    )
    .ok()
}
fn detach(view: MappingId) {
    memory::detach(TeamId::new(0), view, MappingSelection::Whole).unwrap();
}
fn until(mut ready: impl FnMut() -> bool) {
    let deadline = chrono::clock() + 5_000_000_000;
    while !ready() {
        assert!(chrono::clock() < deadline, "progress timeout");
        room::park(1).unwrap();
    }
}
#[unsafe(no_mangle)]
extern "C" fn manager(_args: *const usize, _count: usize) -> ! {
    let _ = writeln!(Serial, "device-manager started");
    sign_contracts();
    pend_contracts();
    clone_contracts();
    unit_control_contracts();
    let (id, identity) = device::scan(0).unwrap();
    assert!(id != 0);
    assert_eq!(identity, 0x11e81234);
    let team = unit::build(ProgramKind::User, unit::self_id()).unwrap();
    let start = core::ptr::addr_of!(__image_start) as usize;
    let end = core::ptr::addr_of!(__image_end) as usize;
    let pages = (end - start) / 4096;
    let code = allocation(end - start);
    let (mapping, destination) = attach(
        code,
        pages,
        TeamId::new(0),
        Placement::lowest(4096).unwrap(),
        6,
    )
    .unwrap();
    unsafe {
        core::ptr::copy_nonoverlapping(
            start as *const u8,
            destination.get() as *mut u8,
            end - start,
        )
    };
    detach(mapping);
    let executable =
        share_view::unseal(code, kernel_api::model::modifiable::Modifiable::Yes).unwrap();
    pend::narrow(executable, kernel_api::model::modifiable::Modifiable::No).unwrap();
    attach(
        code,
        pages,
        team,
        Placement::exact(VirtAddr::new(start), 4096).unwrap(),
        10,
    )
    .unwrap();
    device::claim(id, team).unwrap();
    assert!(
        device::read(id, 0, 4).is_err(),
        "manager cannot impersonate driver"
    );
    assert!(device::status(id).is_err());
    assert!(device::claim(id, TeamId::new(0)).is_err());
    let events = sign::create().unwrap();
    let source = kernel_api::model::sign::SignSource::DeviceRetired { device: id };
    sign::route(events, source).unwrap();
    let pulse = sign::create().unwrap();
    let shared_data = allocation(4096);
    let shared_root =
        share_view::unseal(shared_data, kernel_api::model::modifiable::Modifiable::Yes).unwrap();
    let args = [id, pulse.get(), shared_root.get()];
    let task = unit::spawn(
        team,
        driver as *const () as usize,
        VirtAddr::new(args.as_ptr() as usize),
        args.len(),
        65536,
    )
    .unwrap();
    pend::accord(
        shared_root,
        task,
        kernel_api::model::modifiable::Modifiable::Yes,
        kernel_api::model::identity::Mark::NONE,
    )
    .unwrap();
    assert!(share_view::open(shared_root).is_err());
    let returned = sign::create().unwrap();
    let returned_source = kernel_api::model::sign::SignSource::Inspect {
        task: unit::self_id(),
        token: shared_root,
    };
    sign::route(returned, returned_source).unwrap();
    let delegated =
        sign::delegate(pulse, task, kernel_api::model::sign::SignRights::LISTEN).unwrap();
    assert!(sign::listen(delegated, Wait::POLL).is_err()); // A numeric receipt is not the parent's authority.
    sign::trigger(pulse).unwrap();
    unit::embark(UnitTarget::Team(team)).unwrap();
    unit::embark(UnitTarget::Task(task)).unwrap();
    assert_eq!(
        sign::listen(returned, Wait::AtMost(10000)).unwrap(),
        kernel_api::abi::reply::SignReply::Source {
            source: returned_source,
            fail: None
        }
    );
    sign::close(returned).unwrap();
    sign::release(returned).unwrap();
    let result = unit::join(UnitTarget::Task(task), Wait::AtMost(10000), false).unwrap();
    let JoinReply::Reaped(exit) = result else {
        panic!("child result missing");
    };
    if cfg!(any(feature = "crash", feature = "activation_exit")) {
        assert_eq!(exit.cause, ExitCause::Fault);
    } else {
        assert_eq!(exit.cause, ExitCause::Reap);
        assert_eq!(exit.reason, 0);
    }
    assert_eq!(
        unit::read_result(task).unwrap(),
        if cfg!(any(feature = "crash", feature = "activation_exit")) {
            (0, 0)
        } else {
            (1, 0xCAFE)
        }
    );
    assert_eq!(
        unit::join(UnitTarget::Team(team), Wait::POLL, false).unwrap(),
        JoinReply::Pending
    );
    unit::finish(team).unwrap();
    assert_eq!(
        unit::join(UnitTarget::Team(team), Wait::POLL, false).unwrap(),
        JoinReply::Complete
    );
    assert!(
        unit::spawn(
            team,
            driver as *const () as usize,
            VirtAddr::new(args.as_ptr() as usize),
            args.len(),
            65536
        )
        .is_err()
    );
    let mut restored = None;
    until(|| {
        restored = share_view::open(shared_root).ok();
        restored.is_some()
    });
    assert_eq!(
        unsafe { (restored.unwrap().0.get() as *const usize).read_volatile() },
        0xABCD
    );
    pend::release(shared_root, kernel_api::model::pend::ReleaseMode::Revoke).unwrap();
    memory::release(shared_data).unwrap();
    let _ = writeln!(
        Serial,
        "PASS Pend cross-Unit writer returns after child cleanup"
    );
    unit::join(UnitTarget::Task(task), Wait::POLL, true).unwrap();
    assert!(unit::read_result(task).is_err());
    Serial
        .write_str("PASS unit result=retained admission=sealed completion=stable\n")
        .unwrap();
    // Do not Oust first: Kernel must settle DMA after the last member exits,
    // even while the manager still retains Team/result metadata.
    let event = sign::listen(events, Wait::AtMost(5000)).unwrap();
    assert_eq!(
        event,
        kernel_api::abi::reply::SignReply::Source { source, fail: None }
    );
    assert_eq!(device::observe(id).unwrap(), 4);
    assert!(device::read(id, 0, 4).is_err());
    sign::close(pulse).unwrap();
    sign::release(pulse).unwrap();
    unit::oust(team).unwrap();
    let line = if cfg!(any(feature = "crash", feature = "activation_exit")) {
        "PASS device-units ownership=1 borrow=1 driver-exit=fault settled=1"
    } else {
        "PASS device-units ownership=1 borrow=1 driver-exit=reap settled=1"
    };
    Serial.write_str(line).unwrap();
    room::reap(0, VirtAddr::new(0), 0)
}
extern "C" fn driver(args: *const usize, count: usize) -> ! {
    assert_eq!(count, 3);
    let foreign_pend =
        kernel_api::model::identity::PendId::from_bytes(&unsafe { *args.add(2) }.to_le_bytes())
            .unwrap();
    assert!(share_view::open(foreign_pend).is_err());
    let mut info = [0usize; 8];
    assert_eq!(
        pend::collect(
            kernel_api::model::identity::PendId::NONE,
            VirtAddr::new(info.as_mut_ptr() as usize),
            1
        )
        .unwrap(),
        1
    );
    let incoming = kernel_api::model::identity::PendId::from_bytes(&info[0].to_le_bytes()).unwrap();
    let (address, _) = share_view::open(incoming).unwrap();
    unsafe {
        (address.get() as *mut usize).write_volatile(0xABCD);
    }
    // Exit cleanup must return this live mapping, including crash paths.

    let id = unsafe { *args };
    let foreign = kernel_api::model::sign::SignId::new(unsafe { *args.add(1) });
    assert!(sign::listen(foreign, Wait::POLL).is_err());
    let (raw, rights) = sign::next(0).unwrap();
    assert_eq!(
        rights,
        kernel_api::model::sign::SignRights::LISTEN.bits() as usize
    );
    let event = kernel_api::model::sign::SignId::new(raw);
    assert!(sign::trigger(event).is_err());
    assert_eq!(
        sign::listen(event, Wait::POLL).unwrap(),
        kernel_api::abi::reply::SignReply::Triggered
    );
    sign::consume(event).unwrap();
    sign::release(event).unwrap();
    Serial
        .write_str("PASS sign boundary=parent-child numeric-handle=not-authority\n")
        .unwrap();
    assert!(device::scan(0).is_err(), "driver has no manager authority");
    assert_eq!(device::inspect(id, 0).unwrap(), 0x11e81234);
    for offset in [4, 0x10, 0x34, 0x40, 0x100] {
        assert!(device::inspect(id, offset).is_err());
    }
    assert_eq!(device::read(id, 0, 4).unwrap().0 & 0xffff, 0xed);
    assert!(device::read(id, 1, 4).is_err());
    assert!(device::read(id, 1 << 20, 4).is_err());
    let source = allocation(4096);
    let (mapping, address) = attach(
        source,
        1,
        TeamId::new(0),
        Placement::lowest(4096).unwrap(),
        6,
    )
    .unwrap();
    unsafe {
        (address.get() as *mut u8).write_bytes(0x33, 4096);
        (address.get() as *mut u8).write_bytes(0x5a, 64);
    };
    assert!(
        device::publish(id, source, 0x400000, true).is_err(),
        "CPU alias must prevent DMA"
    );
    detach(mapping);
    use kernel_api::model::{
        identity::Mark,
        modifiable::Modifiable::{No, Yes},
        pend::{PendRange, ReleaseMode},
    };
    let root = share_view::unseal(source, Yes).unwrap();
    let dma = pend::derive(root, Yes, Mark::NONE, PendRange::Inherit).unwrap();
    let dma_source = memory::acquire(
        AcquireSource::Source {
            token: dma,
            offset: 0,
            count: 1,
        },
        kernel_api::model::unit::ControlId::NONE,
    )
    .unwrap();
    // A failed publication must return the device reborrow to its parent Pend.
    assert!(device::publish(id, dma_source, 3, true).is_err());
    share_view::open(dma).unwrap();
    share_view::shut(dma).unwrap();
    let cpu_reader = if cfg!(feature = "readonly") {
        pend::narrow(dma, No).unwrap();
        Some(share_view::open(dma).unwrap().0)
    } else {
        None
    };
    device::publish(id, dma_source, 0x400000, !cfg!(feature = "readonly")).unwrap();
    assert!(
        attach(
            source,
            1,
            TeamId::new(0),
            Placement::lowest(4096).unwrap(),
            6
        )
        .is_none(),
        "DMA reborrow must suspend root access"
    );
    if let Some(reader) = cpu_reader {
        assert_eq!(unsafe { (reader.get() as *const u8).read_volatile() }, 0x5A);
        share_view::open(dma).unwrap();
    } else {
        assert!(share_view::open(dma).is_err());
    }
    if cfg!(feature = "activation_exit") {
        memory::release(source).unwrap();
        unsafe { core::arch::asm!("unimp", options(noreturn)) };
    }
    until(|| device::status(id).unwrap() == 2);
    let _ = writeln!(Serial, "driver: DMA published");
    let faults = sign::create().unwrap();
    let fault_source = kernel_api::model::sign::SignSource::DeviceFault { device: id };
    sign::route(faults, fault_source).unwrap();
    device::write(id, 0x80, 8, 0x500000).unwrap();
    device::write(id, 0x88, 8, 0x40000).unwrap();
    device::write(id, 0x90, 8, 64).unwrap();
    device::write(id, 0x98, 8, 1).unwrap();
    let fault = sign::listen(faults, Wait::AtMost(5000)).unwrap();
    assert_eq!(
        fault,
        kernel_api::abi::reply::SignReply::Source {
            source: fault_source,
            fail: None
        }
    );
    let (cause, iova) = device::fault(id).unwrap();
    assert!(cause != 0);
    assert_eq!(iova, 0x500000);
    assert_eq!(device::status(id).unwrap(), 2, "FQ is not retirement");
    until(|| device::read(id, 0x98, 4).unwrap().0 & 1 == 0);
    let _ = writeln!(Serial, "driver: hardware fault routed");
    device::write(id, 0x80, 8, 0x400000).unwrap();
    device::write(id, 0x88, 8, 0x40000).unwrap();
    device::write(id, 0x90, 8, 64).unwrap();
    device::write(id, 0x98, 8, 1).unwrap();
    until(|| device::read(id, 0x98, 4).unwrap().0 & 1 == 0);
    let _ = writeln!(Serial, "driver: first DMA complete");
    device::write(id, 0x80, 8, 0x40000).unwrap();
    device::write(id, 0x88, 8, 0x400080).unwrap();
    device::write(id, 0x90, 8, 64).unwrap();
    device::write(id, 0x98, 8, 3).unwrap();
    if cfg!(feature = "crash") {
        memory::release(source).unwrap();
        unsafe { core::arch::asm!("unimp", options(noreturn)) };
    }
    until(|| device::read(id, 0x98, 4).unwrap().0 & 1 == 0);
    let _ = writeln!(Serial, "driver: second DMA complete");
    assert!(
        pend::release(dma, ReleaseMode::Revoke).is_err(),
        "published DMA must retain its Pend responsibility"
    );
    until(|| device::status(id).unwrap() == 4);
    until(|| pend::release(dma, ReleaseMode::Revoke).is_ok());
    memory::release(dma_source).unwrap();
    let _ = writeln!(Serial, "PASS Pend revocation drives Kernel DMA retirement");
    let _ = writeln!(Serial, "driver: Kernel settled");
    let (_, address) = attach(
        source,
        1,
        TeamId::new(0),
        Placement::lowest(4096).unwrap(),
        6,
    )
    .unwrap();
    for i in 0..4096 {
        assert_eq!(
            unsafe { ((address.get() + i) as *const u8).read_volatile() },
            if i < 64 || (!cfg!(feature = "readonly") && (128..192).contains(&i)) {
                0x5a
            } else {
                0x33
            }
        );
    }
    unit::return_(0xCAFE)
}

fn sign_contracts() {
    use kernel_api::{
        abi::reply::SignReply,
        model::sign::{SignRights, SignSource},
    };
    let event = sign::create().unwrap();
    let route = sign::create().unwrap();
    let listener = sign::delegate(event, unit::self_id(), SignRights::LISTEN).unwrap();
    let trigger = sign::delegate(event, unit::self_id(), SignRights::TRIGGER).unwrap();
    assert!(sign::trigger(listener).is_err());
    assert!(sign::listen(trigger, Wait::POLL).is_err());
    assert!(sign::route(listener, SignSource::Sign { token: event }).is_err());
    assert!(sign::delegate(listener, unit::self_id(), SignRights::TRIGGER).is_err());
    sign::trigger(trigger).unwrap();
    assert!(sign::trigger(trigger).is_err()); // No overwrite of pending control state.
    assert_eq!(
        sign::listen(listener, Wait::POLL).unwrap(),
        SignReply::Triggered
    );
    // Register after completion: source state still delivers the event.
    let source = SignSource::Sign { token: listener };
    sign::route(route, source).unwrap();
    assert_eq!(
        sign::listen(route, Wait::POLL).unwrap(),
        SignReply::Source { source, fail: None }
    );
    sign::consume(listener).unwrap();
    assert_eq!(
        sign::listen(route, Wait::AtMost(1)).unwrap(),
        SignReply::Pending
    );
    sign::release(listener).unwrap();
    assert!(matches!(
        sign::listen(route, Wait::POLL).unwrap(),
        SignReply::Source { fail: Some(_), .. }
    ));
    sign::close(event).unwrap();
    assert!(sign::trigger(trigger).is_err());
    sign::release(trigger).unwrap();
    sign::release(event).unwrap();
    sign::close(route).unwrap();
    sign::release(route).unwrap();
    Serial.write_str("PASS sign rights=separate pending=retained route=late-registration release=observable\n").unwrap();
}

fn pend_contracts() {
    use kernel_api::model::{
        identity::Mark,
        modifiable::Modifiable::{No, Yes},
        pend::{PendRange, ReleaseMode},
    };
    let data = allocation(4096);
    let root = share_view::unseal(data, Yes).unwrap();
    let (first, _) = share_view::open(root).unwrap();
    unsafe {
        (first.get() as *mut usize).write_volatile(0x1234);
    }
    let a = pend::derive(root, No, Mark::NONE, PendRange::Inherit).unwrap();
    let b = pend::derive(root, No, Mark::NONE, PendRange::Inherit).unwrap();
    assert!(share_view::open(root).is_err());
    assert!(pend::derive(root, Yes, Mark::NONE, PendRange::Inherit).is_err());
    assert!(pend::derive(a, Yes, Mark::NONE, PendRange::Inherit).is_err());
    let (read_a, _) = share_view::open(a).unwrap();
    let (read_b, _) = share_view::open(b).unwrap();
    assert_eq!(
        unsafe { (read_a.get() as *const usize).read_volatile() },
        0x1234
    );
    assert_eq!(
        unsafe { (read_b.get() as *const usize).read_volatile() },
        0x1234
    );
    pend::release(a, ReleaseMode::Revoke).unwrap();
    assert!(share_view::open(root).is_err());
    pend::release(b, ReleaseMode::Revoke).unwrap();
    let (writer, _) = share_view::open(root).unwrap();
    unsafe {
        (writer.get() as *mut usize).write_volatile(0x5678);
    }
    assert!(pend::derive(root, No, Mark::NONE, PendRange::pages(1, 1).unwrap()).is_err());
    // Validation failure must preserve both parent permission and its live mapping.
    unsafe {
        (writer.get() as *mut usize).write_volatile(0x5678);
    }
    let child = pend::derive(root, Yes, Mark::NONE, PendRange::Inherit).unwrap();
    assert!(pend::derive(root, No, Mark::NONE, PendRange::Inherit).is_err());
    let (writer, _) = share_view::open(child).unwrap();
    unsafe {
        (writer.get() as *mut usize).write_volatile(0x9ABC);
    }
    let child_source = memory::acquire(
        AcquireSource::Source {
            token: child,
            offset: 0,
            count: 1,
        },
        kernel_api::model::unit::ControlId::NONE,
    )
    .unwrap();
    // Unseal on a borrowed source must reborrow the same View, never mint a second root.
    let grandchild = share_view::unseal(child_source, No).unwrap();
    assert!(pend::same(root, grandchild).unwrap());
    memory::release(child_source).unwrap();
    assert!(share_view::open(child).is_err());
    let (reader, _) = share_view::open(grandchild).unwrap();
    assert_eq!(
        unsafe { (reader.get() as *const usize).read_volatile() },
        0x9ABC
    );
    pend::release(child, ReleaseMode::Revoke).unwrap();
    assert!(share_view::open(grandchild).is_err());
    let (writer, _) = share_view::open(root).unwrap();
    assert_eq!(
        unsafe { (writer.get() as *const usize).read_volatile() },
        0x9ABC
    );
    pend::narrow(root, No).unwrap();
    let reader = pend::derive(root, No, Mark::NONE, PendRange::Inherit).unwrap();
    share_view::open(root).unwrap();
    share_view::open(reader).unwrap();
    assert!(pend::narrow(root, Yes).is_err());
    pend::release(root, ReleaseMode::Revoke).unwrap();
    assert!(share_view::open(reader).is_err());
    memory::release(data).unwrap();
    let _ = writeln!(
        Serial,
        "PASS Pend readers, writer, nested reborrow and hardware return"
    );
}
fn clone_contracts() {
    use kernel_api::model::{
        clone_view::{CloneLimits, Oversize, PullOutcome},
        identity::Mark,
        pend::ReleaseMode,
    };
    use kernel_client::call::clone_view;
    let send = allocation(8192);
    let receive = allocation(8192);
    let (send_map, send_at) =
        attach(send, 2, TeamId::new(0), Placement::lowest(4096).unwrap(), 6).unwrap();
    let (receive_map, receive_at) = attach(
        receive,
        2,
        TeamId::new(0),
        Placement::lowest(4096).unwrap(),
        6,
    )
    .unwrap();
    let view = clone_view::unseal(
        Mark::NONE,
        CloneLimits {
            max_len: 5000,
            max_messages: 2,
            max_bytes: 10000,
        },
    )
    .unwrap();
    unsafe {
        core::ptr::write_bytes(send_at.get() as *mut u8, 0x5A, 5000);
    }
    unsafe {
        core::ptr::write_bytes(receive_at.get() as *mut u8, 0, 5000);
    }
    clone_view::push(view, send_at, 5000).unwrap();
    unsafe {
        core::ptr::write_bytes(send_at.get() as *mut u8, 0xA5, 5000);
    }
    let outcome = clone_view::pull(view, receive_at, 5000, Oversize::Keep).unwrap();
    assert!(matches!(outcome, PullOutcome::Received { len: 5000, .. }));
    for index in 0..5000 {
        assert_eq!(
            unsafe { ((receive_at.get() + index) as *const u8).read_volatile() },
            0x5A
        );
    }
    pend::release(view, ReleaseMode::Revoke).unwrap();
    detach(send_map);
    detach(receive_map);
    memory::release(send).unwrap();
    memory::release(receive).unwrap();
    let _ = writeln!(
        Serial,
        "PASS Clone copied Cells remain independent of sender"
    );
}

fn unit_control_contracts() {
    let sibling = unit::spawn(
        TeamId::new(0),
        control_sibling as *const () as usize,
        VirtAddr::new(0),
        0,
        16384,
    )
    .unwrap();
    let args = [sibling.get()];
    let probe = unit::spawn(
        TeamId::new(0),
        control_probe as *const () as usize,
        VirtAddr::new(args.as_ptr() as usize),
        1,
        16384,
    )
    .unwrap();
    unit::embark(UnitTarget::Task(probe)).unwrap();
    assert!(matches!(
        unit::join(UnitTarget::Task(probe), Wait::AtMost(5000), false).unwrap(),
        JoinReply::Reaped(_)
    ));
    assert_eq!(unit::read_result(probe).unwrap(), (1, 7));
    unit::join(UnitTarget::Task(probe), Wait::POLL, true).unwrap();
    unit::embark(UnitTarget::Task(sibling)).unwrap();
    assert!(matches!(
        unit::join(UnitTarget::Task(sibling), Wait::AtMost(5000), true).unwrap(),
        JoinReply::Reaped(_)
    ));
    let _ = writeln!(Serial, "PASS unit membership does not grant peer control");
}
extern "C" fn control_sibling(_: *const usize, _: usize) -> ! {
    unit::return_(9)
}
extern "C" fn control_probe(args: *const usize, count: usize) -> ! {
    assert_eq!(count, 1);
    let sibling = kernel_api::model::identity::TaskId::new(unsafe { *args });
    assert!(unit::embark(UnitTarget::Task(sibling)).is_err());
    assert!(unit::debark(UnitTarget::Task(sibling)).is_err());
    assert!(unit::slay(UnitTarget::Task(sibling)).is_err());
    assert!(unit::join(UnitTarget::Task(sibling), Wait::POLL, false).is_err());
    assert!(unit::read_result(sibling).is_err());
    unit::return_(7)
}
