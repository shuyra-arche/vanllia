# Paired syscall cost

This optional benchmark packages only System and `benchmark.syscall-cost`. It is
unselected by the default product and has no ordinary integration `test.toml`.
It warms up 1,000 SelfId calls, then times exactly 100,000 calls using the guest
monotonic clock. Each result contributes to a checked task-ID checksum. Boot,
measurement output and shutdown are outside the measured interval.

The guest emits one versioned, fixed-layout measurement frame (GS `VNM1:`, four
little-endian u64 values encoded as hex). The shared Rust codec checks size,
encoding, iteration count and checksum. The host also binds the reported task
to System's native Unit binding. Measurements cannot supply a passing native
verdict: zero exit, all retirements, successful System completion and VM exit
zero are independently required.

Build the guest using the independent CLI:

```sh
vanllia build --offline --unit benchmark.syscall-cost
```

Build the two Kernel revisions in isolated checkouts with the same toolchain,
target flags, release profile and `--locked` dependency input. Record the exact
ELF SHA-256 and length for each; do not swap sources in an active checkout.

Provide these explicit inputs to the ignored Rust host target:

| Environment variable | Input |
| --- | --- |
| `VANLLIA_BENCH_ARTIFACT` | Existing benchmark artifact directory |
| `VANLLIA_TEST_FIRMWARE` | Installed firmware file |
| `VANLLIA_TEST_FIRMWARE_SHA256`, `VANLLIA_TEST_FIRMWARE_BYTES` | Firmware identity |
| `VANLLIA_BASELINE_KERNEL` | Baseline Kernel ELF |
| `VANLLIA_BASELINE_SHA256`, `VANLLIA_BASELINE_BYTES` | Baseline ELF identity |
| `VANLLIA_CURRENT_KERNEL` | Current Kernel ELF |
| `VANLLIA_CURRENT_SHA256`, `VANLLIA_CURRENT_BYTES` | Current ELF identity |

```sh
cargo test --manifest-path tests/product-lifecycle/Cargo.toml \
  --test cost --target x86_64-unknown-linux-gnu --locked --offline \
  -- --ignored --nocapture
```

The host snapshots the same initrd and firmware with each Kernel, then runs five
pairs at one CPU, alternating order between pairs. Every run has a 60-second
deadline. Exact input receipts and `samples.json` are saved under
`.vanllia/benchmarks/`; per-run native verdicts, serial and process evidence live
under `.vanllia/runs/`. A failed native verdict fails the test and remains in the
evidence. There is no speed threshold. Compare medians and ranges; this small
QEMU workload cannot establish hardware or general application performance.

The recorded event-log comparison uses Kernel `3aa0a155` versus `44e33a6e`, with
the same Cargo lock SHA-256
`a20561ccd029b98084d96f5b7bbe24eaf67ad2ba8fcd54ddc1274904c5da3653`.
Both compiled with installed nightly-2026-10-09, default features, release and
`riscv64gc-unknown-none-elf`, using `-Crelocation-model=static`,
`-Cforce-frame-pointers=yes`, `-Ccode-model=medium`. Unlike the CLI's product
Cargo overlay, these two direct Kernel builds use the unchanged pinned Env
dependency; their ELF hashes are therefore recorded separately from the product
acceptance Kernel. Full inputs, samples and summary are in
[runtime-results.json](../../docs/runtime-results.json).
