# Exact product artifact acceptance

This Rust host test boots an existing product artifact unchanged: it adds no
test Unit and does not rebuild initrd. It derives required startup identities
from the product's declarations, then sends terminal EOF after native readiness
and a terminal attachment interval. It requires the natural-ending Unit's native
zero exit, every instance retired, successful System completion and VM exit zero.
Human console prose is diagnostic only. A deadline or host-forced stop fails.

The test is ignored by default because it executes QEMU. Select the existing
artifact and an installed firmware file:

```sh
export VANLLIA_TEST_ARTIFACT=/absolute/path/to/artifact-directory
export VANLLIA_TEST_FIRMWARE=/absolute/path/to/fw_dynamic.bin
export VANLLIA_TEST_FIRMWARE_SHA256=verified-sha256
export VANLLIA_TEST_FIRMWARE_BYTES=verified-length
export VANLLIA_TEST_CPUS=4 # optional; defaults to 1, accepts 1 through 4
cargo test --manifest-path tests/product-lifecycle/Cargo.toml \
  --test product --target x86_64-unknown-linux-gnu --locked -- --ignored --nocapture
```

Each run snapshots verified kernel/initrd/firmware
bytes under `.vanllia/runs/product-*` and records `input.json`, serial output,
process evidence and `verdict.json`. The monitor is a failure cleanup mechanism;
using it to force shutdown cannot satisfy the verdict.

The current 11-Unit/17-image product passed at 1, 2 and 4 CPUs using QEMU 10.0.13
and OpenSBI 1.6.
This covers startup and terminal EOF cleanup. It does not exercise authenticated
Shell creation or old scenarios. The separate ignored `cost` target measures
[paired syscall cost](../syscall-cost/README.md); it is not part of this lifecycle test.
