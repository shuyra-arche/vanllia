extern crate alloc;
use kernel_api::boot::platform::riscv as hart_id;
use platform_riscv::{
    Region, harts as hart_facts, interrupt as interrupt_plan, memory as memory_facts,
    resources as plan, topology, tree as description,
};
mod authority;
mod harts;
mod interrupt;
mod memory;
mod routing;
mod translation;
mod tree;
