# Kernel IOMMU platform fixture

This standalone boot image supplies the trusted QEMU virt / EDU preparation and
platform ordering proof for Kernel's Cell I/O acceptance tests. PCI configuration
belongs here; the production `platform/riscv` package supplies firmware parsing and
ordinary boot assembly. The Kernel dependency provides translation, invalidation, queue
shutdown and Page retention. It is separate from Vanllia's product selection.

`Cargo.toml` pins both Kernel and its API to the same provider commit. The Kernel
library supplies the RV64 entry and exports its linker script through Cargo
metadata. The image does not include private files from a neighboring checkout.

## Contract

The supported setup is coherent QEMU virt with `iommu-sys=on`, immutable bus-zero
routing and one EDU at `1.0`. Preparation checks firmware routing, the identity
window and endpoint identity before configuring its BAR. Kernel subsequently
validates normalized controller/source identities and resource authority before exporting a Pend. Bus Master
is enabled while DDTP is Off and stays enabled through retirement.

The unsafe `Quiescence` provider runs only after Kernel has revoked translation,
finished invalidation/fences, held DDTP Off and stopped its queues. It returns
Pending twice, then performs a configuration read to order QEMU's synchronous
device callbacks. Late callbacks still encounter IOMMU denial. The provider
checks the requested controller and requester, and returns the exact opaque
retirement token; the Kernel rejects an old or mismatched receipt.

This reasoning applies only to the emulated topology above. Physical devices,
passthrough, hotplug, bypass, ATS/PRI and noncoherent configurations need their own
verified platform contract. A device completion or a cleared Bus Master bit is
not a Page reclamation receipt.

## Verification

Select a nightly with the RV64 target installed. From Vanllia's root:

```sh
cargo +nightly fmt --manifest-path tests/platform/qemu/Cargo.toml -- --check
cargo +nightly test --locked --manifest-path tests/platform/qemu/Cargo.toml \
  --no-default-features --target x86_64-unknown-linux-gnu --test pci
cargo +nightly build --locked --manifest-path tests/platform/qemu/Cargo.toml \
  --target riscv64gc-unknown-none-elf
```

The host PCI cases check endpoint preparation and failure boundaries; they do
not establish hardware completion. To run the real Kernel scenarios, check out
Kernel at the pinned revision, then run from that checkout:

```sh
export QEMU_RISCV64=/absolute/path/to/qemu-system-riscv64
export QEMU_OPENSBI=/absolute/path/to/fw_dynamic.bin
export KERNEL_PLATFORM_MANIFEST=/absolute/path/to/vanllia/tests/platform/qemu/Cargo.toml
cargo +nightly test --locked -p kernel-memory-tests --test hardware \
  --target x86_64-unknown-linux-gnu cell_io -- --ignored --nocapture
```

QEMU 10.0.13 was used for acceptance. Both ordinary withdrawal and creator exit
run in Debug with 1/4 Harts and Release with 4 Harts; ordinary withdrawal also
checks sparse Hart IDs. The runner records serial output, binary hashes and
commands under Kernel's ignored `server/tests/guest/cell/target/evidence/`.
The separate Kernel `iommu_on_real_qemu_registers` suite covers delayed old
accesses and physical Page reuse directly.

The PCI access/endpoint implementation and its three regression cases were
moved from Kernel revision `00f221f7c89b7af1cca4267dc0b0855ba2ff695b` while removing
Kernel's production PCI control dependency. Related:
[Kernel #80](https://github.com/shuyra-arche/kernel/issues/80).
