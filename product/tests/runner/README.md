# Product Native verification

This runner belongs to the selected Shuyra ABI13 product composition, not to
Vanllia's generic CLI or its Host regression suite. It intentionally remains a
separate Cargo workspace for byte-accurate Native/ABI verification.

This standalone Cargo workspace is a Host-only test utility outside the
production and RV64 workspaces. It reuses the existing
`vanllia-build::workflow::process` for bounded processes, cleanup and execution
evidence, and the production `vanllia_build::boot_capsule` encoder for guest
payload bytes. Local test-only helpers handle exact Git archives, provenance and
the outer initrd envelope, including ABI 10/12 rejection inputs.
It does **not** source or depend on Kernel's private `kernel-verification`
Host library. The Kernel owner maintains its target test runner separately.

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
[integration workflow](../../../.github/workflows/memory.yml) directly selects
five single-Hart Native/ABI/std composition QEMU cases, each with separate
evidence. A pinned
[Kernel-owned reusable workflow](https://github.com/shuyra-arche/kernel/blob/main/.github/workflows/pinned-target.yml)
selects and executes six separate single-Hart Activity, Unsupported, Stopped
and IRQ cases: Vanllia passes only the Kernel SHA from its product gitlink,
not Kernel-private runner commands, fixture revisions or scenario matrices.
Kernel also owns the separate
[IOMMU target QEMU workflow](https://github.com/shuyra-arche/kernel/blob/main/.github/workflows/iommu-target.yml).
System's own native CI checks its Runtime, Supply, RPC and handoff contracts.
Ordinary CI does not claim SMP or Realtime qualification; manual owner-side
execution remains available. Opt-in paired Room/IRQ performance comparisons
still run with four Harts and retain baseline evidence.
The Kernel tested by the workflow is selected from the product's committed
`components/kernel` gitlink, which must agree with its manifests and lockfiles.
There is no separate test-only Kernel version selection. Private Kernel probes
still use the immutable published Vanllia fixture
`f2dfa14f5b1f7bd47ee84841ba5c7112e8475950`; changes to these test inputs
are distinct from ABI migration.

The optional [Kernel-owned Room/IRQ comparison workflow](https://github.com/shuyra-arche/kernel/blob/main/.github/workflows/room-comparison.yml)
owns its five private experiment variants, four-Hart release profiles, five
rounds, immutable control revision, alternating boots, and raw evidence.
Vanllia only gates the requested run and passes its selected product Kernel
SHA. A caller PR must include `Room comparison: requested` and exactly one
`Room baseline: <40 lowercase hex characters>` line. The comparison is not
ordinary CI or product acceptance; samples from a pair are not run concurrently.

Outputs must be new directories. The Vanllia Host executor preserves
`execution.json`, `stdout.log`, `stderr.log`, `exit.json` and a transport
`result.json`; the Native scenario additionally retains `boot/serial.log`
and its own accepted/rejected verdict in `boot/result.json`. An expected
nonzero ABI10/12 rejection is a valid **scenario** only when every negative
assertion is observed: the transport's nonzero exit is not itself a pass.
Timeout, cancellation, unsettled descendants and evidence-write failures
remain errors, not accepted guest exits. The Host executor kills and reaps its
process group on failure and checks for residual children after normal exit.
Input checkouts are read through committed Git archives; uncommitted edits
are not silently included. Builds preserve original/resolved locks, commands,
compiler/emulator/firmware identity and image hashes.

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
