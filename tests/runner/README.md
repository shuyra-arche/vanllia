# Native verification

This standalone Cargo workspace is a host utility outside the product and RV64
workspaces. It uses Kernel's small host execution library for bounded processes,
evidence and Capsule encoding. Kernel does not depend on Vanllia's tooling.
The shared orchestration contracts run in Kernel's own Host CI.

`verify native` owns the current memory/IPC/Swap/Virtqueue composition, including
ABI rejection; see [the memory guide](../memory/README.md). `verify versions`
checks the selected product's coordinated Kernel manifests, locks and gitlink.
Other Vanllia tooling has not been migrated by this change.

The System runtime, Supply and RPC guest and its assertions are owned and run
by [System's native verification](https://github.com/shuyra-arche/system/blob/main/.github/workflows/native.yml).
This runner no longer contains a second, fixed System/Twine version pair or
a System-private scenario. The remaining native ABI composition does not claim
complete System or physical hardware acceptance.

## CI ownership and parallelism

Kernel CI only runs its own Host checks and parallel debug/release RV64 builds.
Vanllia owns cross-provider composition testing. The
[integration workflow](../../.github/workflows/memory.yml) selects 11
single-Hart contract/ABI/QEMU jobs, up to eight in parallel, with `fail-fast: false`
and separate evidence per job. Kernel's standalone
[IOMMU target QEMU workflow](https://github.com/shuyra-arche/kernel/blob/main/.github/workflows/iommu-target.yml)
owns its MMIO/DMA assertions and target evidence; Vanllia no longer duplicates
that Kernel-private QEMU invocation. System separately runs its own runtime,
Supply, RPC and handoff contracts; Vanllia no longer duplicates their runtime
scenario. The Kernel-owned SMP, Flat and realtime multi-Hart campaign is not
a routine CI gate; affected tests remain explicitly
runnable by their owners. Opt-in paired Room/IRQ performance comparisons run with
four Harts and retain reproducible baseline evidence.
The Kernel tested by the workflow is selected from the product's committed
`components/kernel` gitlink, which must agree with its manifests and lockfiles.
There is no separate test-only Kernel version selection. Private Kernel probes
still use the immutable published Vanllia fixture
`f2dfa14f5b1f7bd47ee84841ba5c7112e8475950`; changes to these test inputs
are distinct from ABI migration.

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

The ABI 13 native composition pins Kernel
`789ffd19b5359f3de172fb9aad8a61f6a7113a64`. The separate optional std
consumer still selects an immutable System revision for its real Loader; this
does not grant the Vanllia test runner ownership of System's native business
assertions. There is no ABI upgrade or provider pin change in this cleanup.
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

The earlier `6fe567ab8b7532772ec34ecc7ab803282d3f30c8` test-only
override changed only the Realtime test admission retry and is no longer
selected by the ordinary integration workflow, which does not run Realtime
qualification. Owner-side Realtime tests and their required Busy/budget
assertions remain available by explicit execution. The product Kernel pin,
ABI and lockfiles are unchanged.
