# Shuyra product verification

This directory owns the **selected Shuyra ABI13 product composition** tests.
Source selections remain in the repository-root `vanllia.toml`,
`vanllia.lock`, pinned Cargo inputs and Git provider links. This layout is
not a requirement for other products using Vanllia's existing `--root`.

- [Native Memory / ABI13 fixture](tests/memory/README.md) protects the
  public Kernel API/Client/Server combination, including IPC, Swap, Virtqueue
  and memory authority.
- [Native Host verification](tests/runner/README.md) owns the positive Native
  and ABI10/12 negative boot verdicts, QEMU evidence and cleanup. Its Guest
  assertions and exact provider revisions are unchanged.
- [Historical product lifecycle probes](tests/lifecycle/README.md)
  remain separately ignored and **have not been validated against the current
  product lock**.

The existing [composition workflow](../.github/workflows/memory.yml) runs five
Vanllia-product Native/ABI/std QEMU cases and calls six Kernel-owned Activity/IRQ
target cases on one Hart. No full product/physical hardware/SMP/Realtime/ABI14
qualification is implied.

The generic CLI, builder and test discovery contracts remain in `tests/cli/`
and `crates/build/tests/`. New provider tests can use public `test.toml` and
`--root` contracts without adding their Guest or assertions to Vanllia.
