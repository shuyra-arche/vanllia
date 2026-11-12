# RISC-V platform image

Vanllia owns this production `platform-riscv` package and its `shuyra` binary.
It decodes firmware, admits bounded boot facts through Kernel API, then assembles
protected controllers and resource authorities through Kernel's trusted Rust boot
interface. This is not a userspace PCI server. No production Kernel dependency
points back to this package.

The current external-translation backend is admitted only for coherent QEMU virt,
one platform IOMMU and immutable routing. Unknown/disabled controllers, bypass,
ATS/PRI, noncoherent hardware and ambiguous topology fail admission. A real board
needs a verified coherence, ownership and completion contract before using it.
The source compatibility boundary is Kernel's explicit
`kernel_api::boot::platform::riscv::{Platform, devices::Devices}` input plus
`ExternalRoute` and `Quiescence`; the syscall ABI is separate. `Memory` and
`Resources` remain common physical admission contracts. `Devices.resources`
contains the MMIO ownership facts; CPU/Sv/Sstc/SBI and PLIC facts stay in RV64.
`Memory::metadata()` is opaque in the common API and interpreted as DTB here.

This platform requires the matching Kernel source pin in both this package and
`tests/platform/qemu`, including their lockfiles. Merge the Kernel provider PR
first, then the linked Vanllia consumer PR. The syscall ABI remains v5; device
dispatch uses Ledger v7 pairs as described below. No second-ISA boot support
is introduced.

`src/memory`, `harts` and `tree` decode bounded firmware facts; `resources`,
`topology` and `interrupt` resolve protected ownership and wiring. `boot` freezes
facts on separate call stacks before enabling allocation. `build.rs` owns the
QEMU load address; Kernel supplies the section-layout script. The normal image
installs no externally grantable requester. The EDU fixture in `tests/platform/qemu`
uses this same boot path, then supplies its exclusively owned resource and proof.

## Verification

```sh
cargo +nightly fmt --manifest-path platform/riscv/Cargo.toml -- --check
cargo +nightly test --locked --manifest-path platform/riscv/Cargo.toml --no-default-features --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --manifest-path platform/riscv/Cargo.toml --target riscv64gc-unknown-none-elf
cargo +nightly build --locked --manifest-path platform/riscv/Cargo.toml --release --target riscv64gc-unknown-none-elf
```

Run Kernel's hardware suite with `KERNEL_BOOT_MANIFEST` pointing here and
`KERNEL_PLATFORM_MANIFEST` pointing to the EDU fixture. Tests cover sparse harts,
multiple RAM banks, reserved/no-map holes and ROOT stack admission, in addition to
CPU and external translation behavior. Use the pinned provider or temporarily
patch **both** Kernel and API to a candidate checkout and refresh the image lock.
Host parsing tests do not prove QEMU execution or physical-hardware ordering.

## Firmware admission

Boot validates all enabled root RAM nodes and `reg` tuples before initializing
either heap. It subtracts the kernel/ROOT stack, DTB, initrd, FDT reservation
map and static `/reserved-memory` regions. Bump uses the largest remaining
interval; Frame admits every remaining interval after excluding Bump's consumed
prefix. Identity mappings omit bank holes and `no-map` pages. MMIO exclusion
continues to conservatively protect the whole RAM envelope.

Early parsing uses fixed storage: at most 16 disjoint RAM ranges, 64 merged
reservations (including boot objects) and 32 tree levels. Exhaustion, conflicting
boot objects, overflowing or malformed descriptions fail boot before allocation.
RAM and DTB addresses must be page aligned; partial reserved pages are withheld.
Dynamic reserved-memory requests and nonempty/missing reserved-memory `ranges`
are rejected. Allocator metadata still indexes the physical envelope, so widely
separated banks can exhaust the bootstrap interval; this reports allocation
failure rather than admitting holes. The firmware must supply a readable FDT
blob; structural validation cannot establish the validity of a raw pointer.

Ordinary MMIO resources are decoded using their immediate parent's cell widths
and translated through every ancestor's `ranges` before boot authority is
published. Empty `ranges` means identity; missing, malformed, ambiguous or
uncovered translations reject the whole plan. A resource must fit one window
at each hop, and its final CPU page coverage must fit the intersection of those
windows. Distinct boot authorities cannot overlap even partially within a page.
Disabled nodes and descendants, CPU identifiers, reserved-memory descriptions
and PCI child registers do not become ordinary MMIO authorities.

Interrupt/timer and IOMMU register pages remain protected, including aliases
and disabled controllers. Their existing hardware backends still require
identity-translated paths. PCI retains its dedicated window and routing rules;
its parent-side windows now also pass through the CPU address conversion.
Boot ledger region names use the resulting CPU physical addresses.

CPU admission keeps firmware Hart IDs separate from dense Kernel slots. The boot
hart always owns slot zero; enabled CPUs receive the remaining slots. SBI HSM,
IPI and remote fences use exact firmware targets, and PLIC selection uses the
hardware ID. Disabled CPUs are not started or included in the capability set.
Duplicate IDs, malformed CPU properties, a missing/disabled boot hart and SBI's
reserved all-harts ID fail boot before allocation. Early CPU storage is bounded
to 256 CPU nodes (including disabled ones), independent of ID magnitude. CPU and
memory facts are each frozen once in separate initialization steps; Platform holds
references to both, preserving the 64 KiB ROOT stack budget.

Each enabled CPU must declare `mmu-type` as Sv39, Sv48 or Sv57 and provide the
RV64GC, CSR, instruction-fence and counter baseline used by the Kernel. The
complete `riscv,isa-base` / `riscv,isa-extensions` pair takes precedence; an
incomplete pair fails. Legacy `riscv,isa` supports the unversioned canonical
single-letter sequence with underscore-separated multi-letter extensions, with
I implying CSR/fence/counters as specified by the deprecated DT binding.
Unknown well-formed extensions do not enable Kernel features. Missing or
unsupported MMU/ISA facts fail explicitly; no capability is guessed from QEMU.
The current policy selects the largest mode common to all admitted CPUs, relying
on Sv57 implying Sv48 and Sv48 implying Sv39. Normal boot no longer probes modes
by writing `satp`. The validated, nonzero `/cpus/timebase-frequency` feeds the
existing clock consumer; SBI runtime capabilities remain SBI's responsibility.

The shared `src/tree` decoder owns checked properties, ancestor status,
phandle identity and MMIO byte/page geometry used by boot grants, IOMMU topology,
PLIC routing and PCI host lookup. Controller backends still require explicit
identity bus paths and one register span; sharing the decoder does not broaden
hardware support. The phandle index rejects duplicate or conflicting identities
across the whole DTB and accepts consistent `linux,phandle` aliases.

PLIC resolves CPU-local interrupt controllers through that identity index and
uses the admitted CPU-node membership retained by HartInfo, instead of rebuilding
CPU admission during routing. Matching node text in another DTB does not identify
an admitted CPU. Supervisor contexts belonging to disabled CPUs remain masked and
cannot become the selected route. Existing QEMU IRQ-pair compatibility and the
firmware routing restrictions remain explicit backend constraints; PCI BAR
placement belongs to the external image.


Hardware interrupt admission normalizes exclusive QEMU virt level-high UART,
virtio-mmio and RTC sources into `(controller, source, MMIO region)` facts. The
Kernel publishes each device's `(Share View, Sign)` pair in Ledger v7. The PLIC
is configured once for driver sources and reserved IOMMU queues. Invalid, shared,
reserved, unknown-controller and MSI/cascade sources fail before publication.
CPU-local CLINT/ACLINT interrupts remain firmware mechanisms. Drivers use existing
Sign Listen/Consume and Pend delegation/revocation; syscall ABI v5 is unchanged.
The platform does not expose PLIC MMIO to userspace. The trusted Rust
`grant_interrupt` now takes the matching region as its third argument.
