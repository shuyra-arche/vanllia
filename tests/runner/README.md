# Native verification

This standalone Cargo workspace is a host utility outside the product and RV64
workspaces. It uses Kernel's small host execution library for bounded processes,
evidence and Capsule encoding. Kernel does not depend on Vanllia's tooling.
The shared orchestration contracts run in Kernel's own Host CI.

`verify native` owns the current memory/IPC/Swap/Virtqueue composition, including
ABI rejection; see [the memory guide](../memory/README.md). `verify versions`
checks the selected product's coordinated Kernel manifests, locks and gitlink.
Other Vanllia tooling has not been migrated by this change.

`verify system --kernel PATH --fixture SYSTEM --twine TWINE --output NEW` owns
the native System/Twine runtime regression. System is pinned to
`91dd6c5d43e00b152855eff0915212eb08a62664` and Twine to
`e547f60b2ed4c3c09c49de3a360210d90d4ad972`; it executes release on 1/4 Harts.
Neither command claims complete System or physical hardware acceptance.

## CI ownership and parallelism

Kernel CI only runs its own Host checks and parallel debug/release RV64 builds.
Vanllia owns cross-provider composition testing. The
[integration workflow](../../.github/workflows/memory.yml) has 31 independent
scenario/profile/Hart jobs, up to eight at once, with `fail-fast: false` and a
separate evidence artifact per job. Native memory, CPU completion, A/D, Flat,
IRQ, RT, System/Twine and the standalone IOMMU backend retain their scenarios.
The Kernel candidate is the full SHA in `tests/runner/kernel`; it is an explicit
test override, not a change to the product's default Kernel pin. Private Kernel
probes use the published Vanllia fixture `f2dfa14f5b1f7bd47ee84841ba5c7112e8475950`.

Long comparisons require `Room comparison: requested` and one exact
`Room baseline: <40 lowercase hex characters>` line in the Vanllia PR body.
Mode/instrumentation combinations run on independent workers. Within one paired
comparison, build order and alternating A/B boots remain controlled; those
samples cannot be run concurrently without changing the experiment.

Outputs must be new directories. Tools preserve process output before checking
acceptance; success requires a normal exit plus complete scenario evidence.
The whole process group is killed and reaped at a deadline. Input checkouts are
read through committed Git archives. Working-tree edits are not silently included.
Builds preserve original/resolved locks, commands, compiler/emulator/firmware
identity and image hashes.

Integration order for Kernel #199: publish/land Kernel's host library and local
verification first, then pin this composition consumer and run the assembled
matrix here. There is no reverse Kernel-to-Vanllia tooling dependency and no
claim that unrelated Vanllia or System scripting has been removed.

The ABI 13 composition pins Kernel 9c0fc4da1ca4e41bd6ddea3cba2b5964c07323d2,
System 91dd6c5d43e00b152855eff0915212eb08a62664 and Twine
e547f60b2ed4c3c09c49de3a360210d90d4ad972. Kernel #207–#211 land first;
Toolchain #19/Twine #12/System #24 follow before this composition consumer.
Native Debug has a bounded 240-second deadline (Release 120); the migrated
fixture retains actual stack owners and retires them after observed Task
reclamation. An earlier Debug run completed its contracts but timed out during
centralized root cleanup. With per-join retirement it completed in 108 seconds.
The full-marker/normal-exit requirement is unchanged; ABI 10 and 12 are separate
negative boot tests. No full System + Toolchain product acceptance is claimed.

The separate std composition job checks out immutable System/Toolchain inputs,
builds the receipt-checked real std-waits ELF and starts it through System's
production Loader on one qualified Hart. Both the normal Loader invariants and
std completion/reclamation/mapping-retirement marker are required; exact source
commits, receipt/ELF/Kernel hashes and native logs are retained.
