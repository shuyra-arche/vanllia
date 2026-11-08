#![no_std]
#![no_main]
#[unsafe(no_mangle)]
extern "C" fn main(hart: usize, dtp: usize) -> ! {
    let initialized = unsafe { platform_riscv::boot::init(hart, dtp) };
    match initialized.and_then(|_| kernel::entry::boot::start()) {
        Ok(()) => kernel::entry::boot::run(),
        Err(error) => kernel::entry::boot::fail(error),
    }
}
