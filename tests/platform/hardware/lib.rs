#![cfg_attr(target_arch = "riscv64", no_std)]
//! Exclusive coherent QEMU machine fixture, never a production Kernel dependency.
#[cfg(target_arch = "riscv64")]
mod edu;
#[cfg(target_arch = "riscv64")]
pub use edu::Edu;
#[cfg(target_arch = "riscv64")]
mod machine {
    use core::fmt::{self, Write};
    ::core::arch::global_asm!(
        ".section .text.entry",
        ".global _start",
        "_start:",
        "csrw mie, zero",
        ".option push",
        ".option norelax",
        "la gp, __global_pointer$",
        ".option pop",
        "la sp, _stack_top",
        "la t0, trap",
        "csrw mtvec, t0",
        "mv a0, a1",
        "tail guest",
    );

    pub struct Uart;
    impl Write for Uart {
        fn write_str(&mut self, value: &str) -> fmt::Result {
            for byte in value.bytes() {
                unsafe { (0x1000_0000 as *mut u8).write_volatile(byte) };
            }
            Ok(())
        }
    }

    pub fn finish(pass: bool) -> ! {
        unsafe { (0x10_0000 as *mut u32).write_volatile(if pass { 0x5555 } else { 0x13333 }) };
        loop {
            unsafe { ::core::arch::asm!("wfi") }
        }
    }
}
#[cfg(target_arch = "riscv64")]
pub use machine::{Uart, finish};
pub struct Environment {
    pub controller: usize,
    pub requester: u32,
    pub scenario: &'static str,
}
/// # Safety
/// DTB is readable for its full encoded extent and remains immutable forever.
pub unsafe fn discover(dtb: usize) -> Environment {
    let tree = unsafe { fdt::Fdt::from_ptr(dtb as *const u8) }.expect("DTB");
    let topology = platform_riscv::topology::Topology::discover(&tree).unwrap();
    let scenario = tree.chosen().bootargs().unwrap_or("");
    let controller = topology.controllers().next().expect("IOMMU missing");
    let requester = if matches!(
        scenario,
        "dma"
            | "dma-change"
            | "readonly"
            | "dma-abandon"
            | "owner-exit"
            | "activation-exit"
            | "route-reject"
    ) {
        let route = topology.resolve(0x30000000, 8).unwrap();
        assert_eq!(route.controller(), controller);
        assert!(topology.resolve(0x30001000, 8).is_err());
        route.device()
    } else {
        0
    };
    let plan = platform_riscv::resources::Plan::discover(&tree, 0x80000000, 128 << 20).unwrap();
    for authority in plan.authorities() {
        assert!(
            !authority.overlaps(platform_riscv::tree::mmio::Region::new(controller, 4096).unwrap())
        );
        for (base, size) in topology.configurations() {
            assert!(
                !authority.overlaps(platform_riscv::tree::mmio::Region::new(base, size).unwrap())
            );
        }
    }
    Environment {
        controller,
        requester,
        scenario,
    }
}

#[cfg(not(target_arch = "riscv64"))]
pub mod host;
