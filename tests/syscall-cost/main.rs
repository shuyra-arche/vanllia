#![no_std]
#![no_main]

mod sample;

#[mold::entry]
fn main() -> Result<(), env::Reason> {
    let task = env::unit::self_id().get() as u64;
    for _ in 0..1000 {
        core::hint::black_box(env::unit::self_id());
    }
    let start = env::chrono::clock();
    let mut checksum = 0u64;
    for _ in 0..sample::ITERATIONS {
        checksum = checksum.wrapping_add(core::hint::black_box(env::unit::self_id()).get() as u64);
    }
    let elapsed_ns = env::chrono::clock()
        .checked_sub(start)
        .ok_or(0x600001usize)?;
    if elapsed_ns == 0 || checksum != task.wrapping_mul(sample::ITERATIONS) {
        return Err(0x600002);
    }
    let bytes = sample::Sample {
        iterations: sample::ITERATIONS,
        elapsed_ns,
        checksum,
        task,
    }
    .encode();
    let n = env::debug::put(env::VirtAddr::new(bytes.as_ptr() as usize), bytes.len())
        .map_err(|_| 0x600003usize)?;
    if n != bytes.len() {
        return Err(0x600004);
    }
    Ok(())
}
