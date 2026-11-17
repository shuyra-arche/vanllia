# Memory ABI 14 candidate

This is a **candidate integration fixture**, not a second product pin or a
replacement for the full ABI 13 native acceptance suite in `tests/memory`.

The workspace uses the exact Kernel source commit
`0f5f1ab614f662f111fbd3e0c7ca9782d60dee3d` and the production Vanllia firmware/boot source through a test-only
`provider` manifest. This avoids changing `platform/riscv`, the product's
`components/kernel` gitlink or its accepted ABI 13 lock files before the
candidate has executable evidence. The boot library is compiled with the
candidate Kernel/API revision; it is not a prebuilt ABI 13 platform.

The native guest exercises real U-mode ECALL: Flat Create and Occupy,
Pend-backed Nest Reserve, fixed-origin Memo, live Source::Flat association,
Publish, CPU read/write and Task exit. It does not yet replace the
Fault/IPC/Swap/Virtqueue, cross-Team, Partial Release or external DMA acceptance
of the historical native suite. Those invariants must be migrated without
deleting their coverage.

Use the existing `tests/runner` host executor, selecting `native14`; this
preserves the single bounded build/boot/evidence owner. The ABI 14 candidate
workflow runs debug/release under QEMU and retains the exact source, build logs,
resolved Cargo lock and serial/exit evidence. A clean build is not a runtime pass.

**Integration order:** Kernel provider first; this candidate QEMU result; then
native scenario migration and tested product pins/locks; finally full
Vanllia/System/Hub and hardware acceptance.
