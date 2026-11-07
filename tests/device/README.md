# Kernel device lifecycle across Units

`cargo test --workspace` discovers this ignored, environment-dependent test.
Use a Kernel built from the same revision as the guest's API/client pins:

```sh
# In the matching Kernel checkout
cargo +nightly build --locked --release -p kernel \
  --features qemu-edu-dma --target riscv64gc-unknown-none-elf

# In Vanllia
SHUYRA_KERNEL_ELF=/absolute/path/to/shuyra \
SHUYRA_FIRMWARE=/absolute/path/to/fw_dynamic.bin \
QEMU_RISCV64=/absolute/path/to/qemu-system-riscv64 \
cargo +nightly test --locked --test device -- --ignored --nocapture
```

The runner builds a minimal capsule through `vanllia-build`, starts the actual
Kernel under OpenSBI with two harts, and runs a trusted test manager plus a U-mode
Driver. The manager uses the public Unit, Memory, Device and Tole interfaces.
Device business code is confined to `guest/`. There is no IOMMU Service or raw
ECAM, PLIC, IOMMU or PCI-aperture grant in this deployment.

Four independent boots cover normal retirement, a write still pending when the
Driver faults, read-only DMA, and exit during publication. The contract checks:

- Device assignment, immutable config access, BAR bounds and non-owner denial.
- CPU aliases prevent DMA publication; a published borrow prevents new CPU
  mappings; rejected pre-publication setup restores source usability.
- Real EDU read/write DMA changes the expected bytes; read-only writes do not.
- A real invalid IOVA produces an FQ interrupt handled by Kernel and a typed
  fault source routed through Sign/Tole. Observing it does not settle the Cell.
- Kernel retirement and notification complete before the manager calls Oust;
  explicit retirement restores legal CPU access only after settlement.

`target/device-evidence/<mode>/` contains the command, artifact hashes, guest,
initrd, serial log and stderr; the parent directory snapshots Kernel and firmware.
A timeout, panic, unexpected exit cause, missing hardware interrupt or missing
success marker fails the test. An ignored test is not a successful execution.

This is a deliberately restricted platform contract: coherent QEMU virt with a
single platform IOMMU, PLIC and emulated EDU, no passthrough/hotplug/ATS/PRI, one
private 4 KiB page, one-shot device/controller assignment. QEMU 10.0.13 was
validated. FQ delivery is exercised; CQ completion still needs bounded polling
on this QEMU version, and PQ remains disabled. IRQ delivery is never itself the
platform quiescence proof. Both read-only and writable DMA currently take an
exclusive source borrow; general shared-read Pend support is separate work.

The device lease ends when the Team's last member exits, independently of the
parent retaining Team/result metadata. This does not define general Team
completion, nor permit assigning the retired device again. The test does not
validate the normal System/PCI/VirtIO product deployment or migrate those older
consumers. Physical reuse and stale-IOVA isolation are separately covered by
Kernel's M-mode hardware suite; this suite does not infer physical identity from
a returned virtual address.
