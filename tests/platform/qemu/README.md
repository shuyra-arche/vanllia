# QEMU IOMMU platform fixture

This standalone boot image retains QEMU virt / EDU preparation and platform
wiring from the historical Kernel Cell I/O acceptance suite. Its PCI Host tests
and image builds remain Platform checks; this image is not selected by Kernel's
current IOMMU target workflow. PCI configuration belongs here; the production
`platform/riscv` package supplies firmware parsing and ordinary boot assembly. The Kernel dependency provides translation, invalidation, queue
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

The unsafe `Quiescence::prove` provider accepts separate Range and Binding
retirements. For Range, Kernel has invalidated the affected leaf and completed
PR/PW fences while other leaves and queues remain live. For Binding, DDTP is Off
and queues have stopped. The provider retains Pending while EDU DMA_RUN says a
queued timer callback has not completed, then performs an ordered configuration
read and returns the exact token after two bounded Pending observations.
A newly supplied token restarts that proof; a previous token cannot settle a new
publication. The trusted manager must remove the old driver's MMIO authority
before reassigning the requester; the provider exclusively retains configuration
control. This platform condition is separate from software generations.

The image registers MMIO with `grant_mmio` and controller/requester/proof with
`bind_external` independently. Ledger v5 gives U-mode tests separate Share and
Port Pends. The tests create their own External Nests, publish two Memos, retract
one independently, replace and republish a Page, retain fault/completion events,
revoke a Port with DMA pending, exit a driver and safely rebind a fresh Nest.

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
not establish hardware completion. Kernel's current [IOMMU target workflow](https://github.com/shuyra-arche/kernel/blob/main/.github/workflows/iommu-target.yml)
runs its own MMIO/DMA scenarios under `server/tests/target/metal/riscv/iommu/`;
it does **not** boot this historical EDU image. The former
`--test hardware cell_io` command, its multi-Hart test matrix and its
`server/tests/guest/cell/target/evidence/` output path are retired. Do not
treat this fixture's Host PCI tests or RV64 compilation as a current Kernel
IOMMU QEMU execution verdict.

The original QEMU 10.0.13 acceptance results are historical evidence for their
specific source and fixture revisions. Current Kernel QEMU evidence comes from
the Kernel-owned workflow and its uploaded artifacts.

The PCI access/endpoint implementation and its three regression cases were
moved from Kernel revision `00f221f7c89b7af1cca4267dc0b0855ba2ff695b` while removing
Kernel's production PCI control dependency. Related:
[Kernel #80](https://github.com/shuyra-arche/kernel/issues/80).
