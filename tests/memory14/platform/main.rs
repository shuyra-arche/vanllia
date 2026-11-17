#![no_std]
#![no_main]
extern crate alloc;

use kernel::entry::boot::{
    self,
    memory::topology::{Fabric, Ram, Topology},
};

#[unsafe(no_mangle)]
extern "C" fn main(hart: usize, dtp: usize) -> ! {
    let result = unsafe { platform_riscv::boot::init(hart, dtp) }.and_then(|_| {
        let tree = unsafe { fdt::Fdt::from_ptr(dtp as *const u8) }
            .map_err(|_| boot::BootError::Description("invalid native14 DTB"))?;
        let ram = tree
            .memory()
            .regions()
            .map(|r| Ram {
                base: r.starting_address as u64,
                bytes: r.size.expect("qualified QEMU RAM") as u64,
            })
            .collect();
        // This candidate VM runs one qualified coherent Hart with no assigned
        // DMA requester. An empty route set makes no device-safety claim.
        let topology = unsafe {
            Topology::new(alloc::vec![Fabric {
                ram,
                routes: alloc::vec![],
            }])
            .map(|topology| topology.with_local_instructions())
        }
        .map_err(|_| boot::BootError::Description("invalid native14 RAM topology"))?;
        boot::qualify_memory(topology)
            .map_err(|_| boot::BootError::Description("invalid native14 RAM qualification"))?;
        boot::start()
    });
    match result {
        Ok(()) => boot::run(),
        Err(error) => boot::fail(error),
    }
}
