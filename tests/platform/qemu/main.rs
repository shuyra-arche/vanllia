#![no_std]
#![no_main]
#![feature(allocator_ext)]
extern crate alloc;

mod pci;
use alloc::boxed::Box;
use kernel::entry::boot::{self, ExternalRoute, HardwareError, MapMmio, Quiescence, Retirement};
use kernel_api::abi::call::pend::PendFail;

const HOST: usize = 0x3000_0000;
const REQUESTER: u16 = 1 << 3;
const BAR: usize = 0x4000_0000;
const EDU: u32 = 0x11e8_1234;

#[unsafe(no_mangle)]
extern "C" fn main(hartid: usize, dtp: usize) -> ! {
    // SAFETY: this image is paired with coherent QEMU virt, one emulated EDU,
    // immutable routing, and no passthrough/hotplug/ATS/PRI/bypass.
    let initialized = unsafe { platform_riscv::boot::init(hartid, dtp) }.and_then(|_| {
        let route = prepare(dtp, boot::map_mmio).map_err(|source| boot::BootError::Resources {
            operation: boot::ResourceOperation::Devices,
            source,
        })?;
        unsafe { boot::bind_external(route) }
    });
    match initialized.and_then(|_| boot::start()) {
        Ok(()) => boot::run(),
        Err(error) => boot::fail(error),
    }
}

fn prepare(dtp: usize, map: MapMmio) -> Result<ExternalRoute, PendFail> {
    // Kernel validated the firmware blob before invoking this trusted callback.
    let tree = unsafe { fdt::Fdt::from_ptr(dtp as *const u8) }.map_err(|_| PendFail::Denied)?;
    if !tree.all_nodes().next().is_some_and(|root| {
        root.compatible().is_some_and(|c| {
            c.all()
                .any(|s| matches!(s, "riscv-virtio" | "riscv-virtio,qemu"))
        })
    }) {
        return Err(PendFail::Denied);
    }
    let route = platform_riscv::topology::Topology::discover(&tree)
        .map_err(|_| PendFail::Denied)?
        .resolve(HOST, REQUESTER)
        .map_err(|_| PendFail::Denied)?;
    let controller = route.controller();
    let requester = route.device();
    let (start, end) = pci::window::bar_window(&tree, HOST)?;
    let config = unsafe { map(HOST + usize::from(REQUESTER) * 4096, 4096)? };
    let mut endpoint =
        pci::Endpoint::discover(unsafe { pci::Ecam::new(config) }).map_err(|_| PendFail::Denied)?;
    if endpoint.identity() != EDU
        || BAR < start
        || BAR
            .checked_add(endpoint.size())
            .is_none_or(|edge| edge > end)
    {
        return Err(PendFail::Denied);
    }
    unsafe { endpoint.place(BAR) }.map_err(|_| PendFail::Denied)?;
    endpoint.decode().map_err(|_| PendFail::Denied)?;
    // DDTP is Off, so enabling bus mastering does not authorize any Page.
    // Leave it enabled through retirement: late callbacks must hit IOMMU denial.
    unsafe { endpoint.activate() }.map_err(|_| PendFail::Denied)?;
    let base = endpoint.base();
    let size = endpoint.size();
    // Provider owns config and proof for the entire one-shot controller lifetime.
    let proof = Box::try_new(Proof {
        controller,
        requester,
        config: unsafe { pci::Ecam::new(config) },
        bar: unsafe { map(base, size)? },
        pending: 0,
        ticket: None,
    })
    .map_err(|_| PendFail::OutOfMemory)?;
    unsafe { boot::grant_mmio(kernel_api::boot::platform::Region::new(base, size)) }
        .map_err(|_| PendFail::Denied)?;
    Ok(ExternalRoute {
        controller,
        requester,
        proof,
    })
}

struct Proof {
    controller: usize,
    requester: u32,
    config: pci::Ecam,
    bar: usize,
    pending: usize,
    ticket: Option<Retirement>,
}

// SAFETY: fixed coherent QEMU EDU only. The invalid leaf (Range) or DDTP Off
// (Binding) and PR/PW fence are already established by Kernel. QEMU's EDU
// dma_timer runs its complete pci_dma callback before clearing DMA_RUN. Reading
// the command register and then config is ordered through device execution;
// a still scheduled timer retains the proof instead of being mistaken for an
// already completed callback. Reassignment requires old driver's MMIO authority
// removed by the trusted manager; config remains exclusively with this provider.
// No hotplug, bypass, ATS/PRI, asynchronous passthrough or real-device claim.
unsafe impl Quiescence for Proof {
    fn prove(&mut self, ticket: Retirement) -> Result<Option<Retirement>, HardwareError> {
        if ticket.controller() != self.controller || ticket.requester() != self.requester {
            return Err(HardwareError::Fault);
        }
        if self.ticket != Some(ticket) {
            self.ticket = Some(ticket);
            self.pending = 2;
        }
        // A delayed callback must finish while its old address is still denied.
        unsafe {
            core::arch::asm!("fence iorw, iorw", options(nostack));
        }
        let command = unsafe { ((self.bar + 0x98) as *const u64).read_volatile() };
        if command & 1 != 0 {
            return Ok(None);
        }
        if self.pending != 0 {
            self.pending -= 1;
            return Ok(None);
        }
        if self.config.read32(0) == EDU {
            Ok(Some(ticket))
        } else {
            Err(HardwareError::Fault)
        }
    }
}
