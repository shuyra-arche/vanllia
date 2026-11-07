#![no_std]
#![no_main]
use core::fmt::{self, Write};
use kernel_api::model::{
    identity::{TeamId, VirtAddr},
    memory::{
        AcquireSource, Allocation, FlatReference, MappingControl, MappingId, MappingSelection,
        MappingState, MappingTarget, Placement, ReferenceRange,
    },
    unit::{ExitCause, JoinReply, ProgramKind, UnitTarget},
    wait::Wait,
};
use kernel_client::call::{chrono, debug, memory, pend, room, share_view, sign, unit};

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
        MappingTarget::Cpu(team),
        VirtAddr::new(place.as_ptr() as usize),
        access,
        MappingControl::Caller,
    )
    .ok()
}
fn detach(view: MappingId) {
    memory::detach(
        MappingTarget::Cpu(TeamId::new(0)),
        view,
        MappingSelection::Whole,
    )
    .unwrap();
}
fn until(mut ready: impl FnMut() -> bool) {
    let deadline = chrono::clock() + 5_000_000_000;
    while !ready() {
        assert!(chrono::clock() < deadline, "progress timeout");
        room::park(1).unwrap();
    }
}
#[unsafe(no_mangle)]
extern "C" fn manager(boot_args: *const usize, count: usize) -> ! {
    let _ = writeln!(Serial, "device-manager started");
    sign_contracts();
    pend_contracts();
    clone_contracts();
    unit_control_contracts();
    assert_eq!(count, kernel_api::boot::args::LEN);
    let args = unsafe { core::slice::from_raw_parts(boot_args, count) };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            args[kernel_api::boot::args::LEDGER] as *const u8,
            args[kernel_api::boot::args::LEDGER_LEN],
        )
    };
    let ledger = kernel_api::boot::ledger::Entries::new(bytes).unwrap();
    let bar_root = kernel_api::model::identity::PendId::from_bytes(
        &ledger
            .find(kernel_api::boot::ledger::Name::Page(
                kernel_api::boot::ledger::Page::Region(0x40000000),
            ))
            .unwrap()
            .handle()
            .to_le_bytes(),
    )
    .unwrap();
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
    let pulse = sign::create().unwrap();
    let shared_data = allocation(4096);
    let shared_root =
        share_view::unseal(shared_data, kernel_api::model::modifiable::Modifiable::Yes).unwrap();
    let clone_queue = kernel_client::call::clone_view::unseal(
        kernel_api::model::identity::Mark::NONE,
        kernel_api::model::clone_view::CloneLimits {
            max_len: 64,
            max_messages: 1,
            max_bytes: 64,
        },
    )
    .unwrap();
    let message = [0x73u8; 64];
    kernel_client::call::clone_view::push(
        clone_queue,
        VirtAddr::new(message.as_ptr() as usize),
        message.len(),
    )
    .unwrap();
    let args = [
        bar_root.get(),
        pulse.get(),
        shared_root.get(),
        clone_queue.get(),
    ];
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
    pend::accord(
        clone_queue,
        task,
        kernel_api::model::modifiable::Modifiable::No,
        kernel_api::model::identity::Mark::NONE,
    )
    .unwrap();
    pend::accord(
        bar_root,
        task,
        kernel_api::model::modifiable::Modifiable::Yes,
        kernel_api::model::identity::Mark::new(0xED),
    )
    .unwrap();
    assert!(share_view::open(bar_root).is_err());
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
    // This Sign reports the shared writer's return, not completion of all child
    // cleanup. Revocation can still contend with the child's other Pend returns.
    until(
        || match pend::release(clone_queue, kernel_api::model::pend::ReleaseMode::Revoke) {
            Ok(()) => true,
            Err(error) if error.source == kernel_api::abi::call::pend::PendFail::Busy => false,
            Err(error) => panic!("Clone queue release: {error:?}"),
        },
    );
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
    let io_mapping = MappingId::from_bytes(
        &unsafe {
            ((restored.unwrap().0.get() + core::mem::size_of::<usize>()) as *const usize)
                .read_volatile()
        }
        .to_le_bytes(),
    )
    .unwrap();
    let events = sign::create().unwrap();
    let source = kernel_api::model::sign::SignSource::MappingRetired {
        token: bar_root,
        mapping: io_mapping,
    };
    sign::route(events, source).unwrap();
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
    assert_eq!(
        memory::inspect(MappingTarget::Io(bar_root), io_mapping)
            .unwrap()
            .state,
        MappingState::Settled
    );
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
    assert_eq!(count, 4);
    let foreign_pend =
        kernel_api::model::identity::PendId::from_bytes(&unsafe { *args.add(2) }.to_le_bytes())
            .unwrap();
    assert!(share_view::open(foreign_pend).is_err());
    let foreign_queue =
        kernel_api::model::identity::PendId::from_bytes(&unsafe { *args.add(3) }.to_le_bytes())
            .unwrap();
    assert!(kernel_client::call::clone_view::pull(foreign_queue, 64).is_err());
    let mut info = [[0usize; 8]; 3];
    assert_eq!(
        pend::collect(
            kernel_api::model::identity::PendId::NONE,
            VirtAddr::new(info.as_mut_ptr() as usize),
            3
        )
        .unwrap(),
        3
    );
    let records = info.map(|words| kernel_api::model::pend::PendInfo::from_words(words).unwrap());
    let incoming = records
        .iter()
        .find(|p| p.kind == kernel_api::model::view::ViewKind::Share && p.mark.get() != 0xED)
        .unwrap()
        .token;
    let queue = records
        .iter()
        .find(|p| p.kind == kernel_api::model::view::ViewKind::Clone)
        .unwrap()
        .token;
    let payload = kernel_client::call::clone_view::pull(queue, 64).unwrap();
    assert_eq!(payload.len, 64);
    assert_ne!(payload.sender, unit::self_id());
    let (copied, size) = share_view::open(payload.token).unwrap();
    assert_eq!(size, 4096);
    for i in 0..size {
        assert_eq!(
            unsafe { ((copied.get() + i) as *const u8).read_volatile() },
            if i < 64 { 0x73 } else { 0 }
        );
    }
    // Leave this independent payload Pend and its mapping live on every exit path.
    let _ = writeln!(
        Serial,
        "PASS Clone cross-Unit mapped payload retained until receiver exit"
    );
    let bar_token = records.iter().find(|p| p.mark.get() == 0xED).unwrap().token;
    let foreign_bar =
        kernel_api::model::identity::PendId::from_bytes(&unsafe { *args }.to_le_bytes()).unwrap();
    assert!(share_view::open(foreign_bar).is_err());
    let (bar, bar_len) = share_view::open(bar_token).unwrap();
    assert_eq!(bar_len, 1 << 20);
    let id = bar.get();
    let (shared_address, _) = share_view::open(incoming).unwrap();
    unsafe {
        (shared_address.get() as *mut usize).write_volatile(0xABCD);
    }
    // Exit cleanup must return this live mapping, including crash paths.

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
    assert_eq!(bar_read(id, 0, 4) & 0xffff, 0xed);
    let _ = writeln!(
        Serial,
        "PASS MMIO uses Pend mapping without device syscalls"
    );
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
        project(bar_token, source, 0x400000, true).is_err(),
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
    assert!(project(foreign_bar, dma_source, 0x400000, true).is_err());
    assert!(project(payload.token, dma_source, 0x400000, true).is_err());
    assert!(project(dma, dma_source, 0x400000, true).is_err()); // RAM is not a route.
    // Failed authority/placement checks must return the reborrow unchanged.
    assert!(project(bar_token, dma_source, 1usize << 38, true).is_err());
    share_view::open(dma).unwrap();
    share_view::shut(dma).unwrap();
    let cpu_reader = if cfg!(feature = "readonly") {
        pend::narrow(dma, No).unwrap();
        Some(share_view::open(dma).unwrap().0)
    } else {
        None
    };
    let io_mapping = project(bar_token, dma_source, 0x400000, !cfg!(feature = "readonly")).unwrap();
    unsafe {
        ((shared_address.get() + core::mem::size_of::<usize>()) as *mut usize)
            .write_volatile(io_mapping.get());
    }

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
    until(|| {
        memory::inspect(MappingTarget::Io(bar_token), io_mapping)
            .unwrap()
            .state
            == MappingState::Active
    });
    assert!(pend::derive(bar_token, No, Mark::NONE, PendRange::Inherit).is_err());
    assert!(pend::narrow(bar_token, No).is_err());
    let _ = writeln!(
        Serial,
        "PASS active projection retains MMIO routing authority"
    );
    let _ = writeln!(Serial, "driver: DMA published");
    let faults = sign::create().unwrap();
    let fault_source = kernel_api::model::sign::SignSource::MappingFault {
        token: bar_token,
        mapping: io_mapping,
    };
    sign::route(faults, fault_source).unwrap();
    bar_write(id, 0x80, 8, 0x500000);
    bar_write(id, 0x88, 8, 0x40000);
    bar_write(id, 0x90, 8, 64);
    bar_write(id, 0x98, 8, 1);
    let fault = sign::listen(faults, Wait::AtMost(5000)).unwrap();
    assert_eq!(
        fault,
        kernel_api::abi::reply::SignReply::Source {
            source: fault_source,
            fail: None
        }
    );
    let info = memory::inspect(MappingTarget::Io(bar_token), io_mapping).unwrap();
    let (cause, iova) = (info.fault_cause, info.fault_address);
    assert!(cause != 0);
    assert_eq!(iova, 0x500000);
    assert_eq!(
        memory::inspect(MappingTarget::Io(bar_token), io_mapping)
            .unwrap()
            .state,
        MappingState::Active,
        "FQ is not retirement"
    );
    until(|| bar_read(id, 0x98, 4) & 1 == 0);
    let _ = writeln!(Serial, "driver: hardware fault routed");
    bar_write(id, 0x80, 8, 0x400000);
    bar_write(id, 0x88, 8, 0x40000);
    bar_write(id, 0x90, 8, 64);
    bar_write(id, 0x98, 8, 1);
    until(|| bar_read(id, 0x98, 4) & 1 == 0);
    let _ = writeln!(Serial, "driver: first DMA complete");
    bar_write(id, 0x80, 8, 0x40000);
    bar_write(id, 0x88, 8, 0x400080);
    bar_write(id, 0x90, 8, 64);
    bar_write(id, 0x98, 8, 3);
    if cfg!(feature = "crash") {
        memory::release(source).unwrap();
        let _ = writeln!(Serial, "driver: writing read-only Clone payload must fault");
        unsafe {
            (copied.get() as *mut u8).write_volatile(0x99);
        }
        panic!("Clone payload write was permitted");
    }
    until(|| bar_read(id, 0x98, 4) & 1 == 0);
    let _ = writeln!(Serial, "driver: second DMA complete");
    if cfg!(feature = "route_revoke") {
        // Revoking routing authority must retire the data borrow too, even while
        // the Task and data Pend are otherwise still live.
        assert!(pend::release(bar_token, ReleaseMode::Revoke).is_err());
        assert!(share_view::open(bar_token).is_err());
        until(|| share_view::open(dma).is_ok());
        share_view::shut(dma).unwrap();
        let _ = writeln!(
            Serial,
            "PASS MMIO Pend revocation retires external projection"
        );
    } else {
        if cfg!(feature = "readonly") {
            memory::detach(
                MappingTarget::Io(bar_token),
                io_mapping,
                MappingSelection::Whole,
            )
            .unwrap();
            let _ = writeln!(Serial, "PASS Memory Detach closes external mapping");
        } else {
            assert!(
                pend::release(dma, ReleaseMode::Revoke).is_err(),
                "published DMA must retain its Pend responsibility"
            );
        }
        until(|| {
            memory::inspect(MappingTarget::Io(bar_token), io_mapping)
                .unwrap()
                .state
                == MappingState::Settled
        });
    }
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
        clone_view::CloneLimits,
        identity::Mark,
        modifiable::Modifiable::{No, Yes},
        pend::{PendRange, ReleaseMode},
    };
    use kernel_client::call::clone_view;
    let send = allocation(8192);
    let (send_map, send_at) =
        attach(send, 2, TeamId::new(0), Placement::lowest(4096).unwrap(), 6).unwrap();
    // Repeat independent roots to exercise map/revoke/reclaim, including page tails.
    for _ in 0..16 {
        let queue = clone_view::unseal(
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
        clone_view::push(queue, send_at, 5000).unwrap();
        unsafe {
            core::ptr::write_bytes(send_at.get() as *mut u8, 0xA5, 5000);
        }
        assert!(clone_view::pull(queue, 4999).is_err());
        let (len, sender, depth) = clone_view::peek(queue).unwrap();
        assert_eq!((len, sender, depth), (5000, unit::self_id(), 1));
        let outcome = clone_view::pull(queue, 5000).unwrap();
        assert_eq!((outcome.len, outcome.sender), (5000, unit::self_id()));
        assert!(clone_view::pull(queue, 5000).is_err());
        assert!(pend::narrow(outcome.token, Yes).is_err());
        assert!(pend::derive(outcome.token, Yes, Mark::NONE, PendRange::Inherit).is_err());
        let reference = memory::acquire(
            AcquireSource::Source {
                token: outcome.token,
                offset: 0,
                count: 2,
            },
            kernel_api::model::unit::ControlId::NONE,
        )
        .unwrap();
        assert!(
            attach(
                reference,
                2,
                TeamId::new(0),
                Placement::lowest(4096).unwrap(),
                6
            )
            .is_none()
        );
        memory::release(reference).unwrap();
        let reader = pend::derive(outcome.token, No, Mark::NONE, PendRange::Inherit).unwrap();
        clone_view::seal(queue).unwrap();
        pend::release(queue, ReleaseMode::Revoke).unwrap();
        let (address, size) = share_view::open(outcome.token).unwrap();
        assert_eq!(size, 8192);
        for index in 0..size {
            assert_eq!(
                unsafe { ((address.get() + index) as *const u8).read_volatile() },
                if index < outcome.len { 0x5A } else { 0 }
            );
        }
        share_view::open(reader).unwrap();
        pend::release(outcome.token, ReleaseMode::Revoke).unwrap();
        assert!(share_view::open(reader).is_err());
    }
    detach(send_map);
    memory::release(send).unwrap();
    let _ = writeln!(
        Serial,
        "PASS Clone copied Cells remain independent of sender"
    );
    let _ = writeln!(
        Serial,
        "PASS Clone mapped delivery retry, read-only, zero tail and independent lifetime"
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

fn project(
    token: kernel_api::model::identity::PendId,
    reference: FlatReference,
    iova: usize,
    modifiable: bool,
) -> Result<MappingId, ()> {
    let range = ReferenceRange::new(reference, 0, 1).ok_or(())?.encode();
    let placement = Placement::exact(VirtAddr::new(iova), 4096)
        .ok_or(())?
        .encode();
    memory::attach(
        VirtAddr::new(range.as_ptr() as usize),
        MappingTarget::Io(token),
        VirtAddr::new(placement.as_ptr() as usize),
        if modifiable { 6 } else { 2 },
        MappingControl::Caller,
    )
    .map(|(id, _)| id)
    .map_err(|_| ())
}
fn bar_read(base: usize, offset: usize, width: usize) -> u64 {
    unsafe {
        core::arch::asm!("fence iorw, iorw");
    }
    let value = unsafe {
        match width {
            4 => ((base + offset) as *const u32).read_volatile() as u64,
            8 => ((base + offset) as *const u64).read_volatile(),
            _ => panic!("width"),
        }
    };
    unsafe {
        core::arch::asm!("fence iorw, iorw");
    }
    value
}
fn bar_write(base: usize, offset: usize, width: usize, value: u64) {
    unsafe {
        core::arch::asm!("fence iorw, iorw");
        match width {
            4 => ((base + offset) as *mut u32).write_volatile(value as u32),
            8 => ((base + offset) as *mut u64).write_volatile(value),
            _ => panic!("width"),
        };
        core::arch::asm!("fence iorw, iorw");
    }
}
