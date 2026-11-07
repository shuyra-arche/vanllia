#![no_std]
#![no_main]
use core::fmt::{self, Write};
use kernel_api::model::{
    identity::{PieToken, TeamId, VirtAddr},
    memory::{
        AcquireSource, Allocation, FlatReference, Placement, ReferenceRange, ViewControl, ViewId,
        ViewSelection,
    },
    unit::{ExitCause, JoinReply, ProgramKind, UnitTarget},
    wait::Wait,
};
use kernel_client::call::{chrono, debug, device, memory, pie, pole, room, tole, unit};

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
        PieToken::NONE,
    )
    .unwrap()
}
fn attach(
    reference: FlatReference,
    count: usize,
    team: TeamId,
    placement: Placement,
    access: u64,
) -> Option<(ViewId, VirtAddr)> {
    let range = ReferenceRange::new(reference, 0, count).unwrap().encode();
    let place = placement.encode();
    memory::attach(
        VirtAddr::new(range.as_ptr() as usize),
        team,
        VirtAddr::new(place.as_ptr() as usize),
        access,
        ViewControl::Caller,
    )
    .ok()
}
fn detach(view: ViewId) {
    memory::detach(TeamId::new(0), view, ViewSelection::Whole).unwrap();
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
    let executable = pole::unseal(code, true).unwrap();
    pie::narrow(
        executable,
        kernel_api::model::permission::Permission::FETCH
            | kernel_api::model::permission::Permission::VEST,
    )
    .unwrap();
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
    let events = tole::unseal(false).unwrap();
    let source = kernel_api::model::tole::ToleEventSource::DeviceRetired { device: id };
    tole::attach(events, source).unwrap();
    let args = [id];
    let task = unit::spawn(
        team,
        driver as *const () as usize,
        VirtAddr::new(args.as_ptr() as usize),
        args.len(),
        65536,
    )
    .unwrap();
    unit::embark(UnitTarget::Team(team)).unwrap();
    unit::embark(UnitTarget::Task(task)).unwrap();
    let result = unit::join(UnitTarget::Task(task), Wait::AtMost(10000), true).unwrap();
    let JoinReply::Reaped(exit) = result else {
        panic!("child result missing");
    };
    if cfg!(any(feature = "crash", feature = "activation_exit")) {
        assert_eq!(exit.cause, ExitCause::Fault);
    } else {
        assert_eq!(exit.cause, ExitCause::Reap);
        assert_eq!(exit.reason, 0);
    }
    // Do not Oust first: Kernel must settle DMA after the last member exits,
    // even while the manager still retains Team/result metadata.
    let event = tole::await_(events, Wait::AtMost(5000)).unwrap();
    assert_eq!(
        event,
        kernel_api::abi::reply::ToleAwaitReply::Source { source, fail: None }
    );
    assert_eq!(device::observe(id).unwrap(), 4);
    assert!(device::read(id, 0, 4).is_err());
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
    assert_eq!(count, 1);
    let id = unsafe { *args };
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
    // Failed pre-publication validation must cancel the borrow completely.
    assert!(device::publish(id, source, 3, true).is_err());
    let (view, _) = attach(
        source,
        1,
        TeamId::new(0),
        Placement::lowest(4096).unwrap(),
        6,
    )
    .unwrap();
    detach(view);
    device::publish(id, source, 0x400000, !cfg!(feature = "readonly")).unwrap();
    assert!(
        attach(
            source,
            1,
            TeamId::new(0),
            Placement::lowest(4096).unwrap(),
            6
        )
        .is_none(),
        "DMA borrow must prevent CPU aliases"
    );
    if cfg!(feature = "activation_exit") {
        memory::release(source).unwrap();
        unsafe { core::arch::asm!("unimp", options(noreturn)) };
    }
    until(|| device::status(id).unwrap() == 2);
    let _ = writeln!(Serial, "driver: DMA published");
    let faults = tole::unseal(false).unwrap();
    let fault_source = kernel_api::model::tole::ToleEventSource::DeviceFault { device: id };
    tole::attach(faults, fault_source).unwrap();
    device::write(id, 0x80, 8, 0x500000).unwrap();
    device::write(id, 0x88, 8, 0x40000).unwrap();
    device::write(id, 0x90, 8, 64).unwrap();
    device::write(id, 0x98, 8, 1).unwrap();
    let fault = tole::await_(faults, Wait::AtMost(5000)).unwrap();
    assert_eq!(
        fault,
        kernel_api::abi::reply::ToleAwaitReply::Source {
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
    device::retire(id).unwrap();
    until(|| device::status(id).unwrap() == 4);
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
    room::reap(0, VirtAddr::new(0), 0)
}
