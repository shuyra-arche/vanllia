#![no_std]
//! RISC-V firmware decoding and boot assembly owned by Vanllia.
extern crate alloc;
pub mod harts;
pub mod interrupt;
pub mod memory;
pub mod resources;
pub mod topology;
pub mod tree;
pub use kernel_api::boot::platform::Region;
#[cfg(all(target_arch = "riscv64", feature = "image"))]
pub mod boot;
