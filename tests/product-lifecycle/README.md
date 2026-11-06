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

This covers startup and terminal EOF cleanup. It does not exercise authenticated
Shell creation or old scenarios. The separate ignored `cost` target measures
[paired syscall cost](../syscall-cost/README.md); it is not part of this lifecycle test.

## Login and requested Shell smoke

The separate ignored `login` target uses the product's existing public Login
fixture from `components/session/units/login/tests/login/lib.rs`. Build only
Login and its necessary dependency closure; Daemon/RTC are not selected. It requires
the startup set without Shell, authenticates, then waits for Shell's native
binding/readiness before evaluating `(+ 314000 159)`. The numeric result must be
`314159`. EOF closes Shell, then Login. Both must have native Reap/zero receipts;
every instance must be retired, System must complete successfully and QEMU must
exit zero naturally. Prompts schedule input and cannot replace native receipts.

Build the minimum artifact, set `VANLLIA_TEST_ARTIFACT` to the printed artifact
directory, and use the same hashed firmware variables above:

```sh
vanllia build --offline --unit login
cargo test --manifest-path tests/product-lifecycle/Cargo.toml \
  --test login --target x86_64-unknown-linux-gnu --locked --offline \
  -- --ignored --nocapture
```

This target always uses one CPU and a bounded 120-second VM deadline. It adds no
test Unit and does not change the supplied artifact. Per-run input snapshots,
serial/process evidence and `verdict.json` live under `.vanllia/runs/login-*`.
It is a new minimum smoke check; no old scenario suite is migrated.

## PCI, IOMMU and VirtIO block discovery

The ignored `drivers` target runs the fixed `test.virtio-blk-discovery` artifact
from the VirtIO provider. It enables QEMU's real emulated platform IOMMU and
adds one modern PCI VirtIO block device backed by a generated, read-only 4 MiB
zero disk. No static BAR, DMA source or hardware resource is fabricated.

The provider probe authenticates System State and the Hub publisher/list face,
then waits for exactly `virtio-blk-pci-1` in the Hub class. The host requires
exactly five instances, all four resident drivers Ready, probe Reap/zero, every
instance retired, successful System completion and natural VM exit zero. It
uses the selected CLI's typed native verifier; prose cannot replace receipts.

```sh
vanllia build --offline --unit test.virtio-blk-discovery
# Set VANLLIA_TEST_ARTIFACT to the printed directory and the same exact
# VANLLIA_TEST_FIRMWARE, _SHA256 and _BYTES inputs described above.
cargo test --manifest-path tests/product-lifecycle/Cargo.toml \
  --test drivers --target x86_64-unknown-linux-gnu --locked --offline \
  -- --ignored --nocapture
```

This target uses one CPU and a 60-second deadline. Each run preserves verified
kernel/initrd/firmware inputs, the generated disk, command arguments, serial,
process receipts and `verdict.json` under `.vanllia/runs/drivers-*`.
It verifies authenticated discovery, observed domain establishment and Hub
enrollment. It does not exercise block requests, queue DMA, data integrity,
recovery, DMA isolation, physical hardware or SMP.
