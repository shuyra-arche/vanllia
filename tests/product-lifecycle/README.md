# Exact product artifact acceptance

This Rust host test boots an existing product artifact unchanged: it adds no
test Unit and does not rebuild initrd. It derives required startup identities
from the product's declarations, then sends terminal EOF after native readiness
and a terminal attachment interval. It requires the natural-ending Unit's native
zero exit, every instance retired, successful System completion and VM exit zero.
Human console prose is diagnostic only. A deadline or host-forced stop fails.

The test is ignored by default because it executes QEMU. With explicit runtime
authorization, select the existing artifact and an installed firmware file:

```sh
export VANLLIA_TEST_ARTIFACT=/absolute/path/to/artifact-directory
export VANLLIA_TEST_FIRMWARE=/absolute/path/to/fw_dynamic.bin
export VANLLIA_TEST_FIRMWARE_SHA256=verified-sha256
export VANLLIA_TEST_FIRMWARE_BYTES=verified-length
cargo test --manifest-path tests/product-lifecycle/Cargo.toml \
  --target x86_64-unknown-linux-gnu --locked -- --ignored --nocapture
```

The owned VM is single-core. Each run snapshots verified kernel/initrd/firmware
bytes under `.vanllia/runs/product-*` and records `input.json`, serial output,
process evidence and `verdict.json`. The monitor is a failure cleanup mechanism;
using it to force shutdown cannot satisfy the verdict.

The current 11-Unit/17-image product passed using QEMU 10.0.13 and OpenSBI 1.6.
This covers startup and terminal EOF cleanup. It does not exercise authenticated
Shell creation, old scenarios, SMP or performance benchmarks.
