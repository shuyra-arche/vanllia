# Device driver intake validation

This intake extends Vanllia main
`86319ef40d34b18aeb6e262162a0831d7e3ba789`, retaining the merged Login/requested
Shell lifecycle work. Default roots remain Login and Daemon. No old scenario
suite is migrated and no kernel cluster is added.

| Source | Fixed revision |
| --- | --- |
| System | `e6089db98cb2e298bb98cd828f00a0129b7f40cd` |
| Driver aggregate | `1431f2058c412fd4f20ef3b81f54bb06ad5cdc6d` |
| PCI | `b2c2e5d7a5fbd0b1572273dbbffb56bd91b15009` |
| IOMMU | `3d210b4da522ee0c7427265ce0a0d8946e124b21` |
| VirtIO block | `58569a4f3033edc699d1ba02ce2f4f660a2480dc` |
| Kernel | `44e33a6ee28d86eeeabe61c225f4176c5ed64c59` |

System's only additional change lists the already-registered
`memory/retention` collection in its generated runtime resource metadata. It is
rebased on System default `3e1ce44d161a700db5a1b7b192114cfc396e7ec5`, so the
requested-instance receipts remain present. Runtime ownership, admission and
permissions are unchanged. The earlier public metadata checkpoint `b22bbfb0...`
predates that merge and is not the selected final System revision.

The Driver aggregate discovers seven production Units and two new provider
probes through exact gitlinks. Its retained legacy registry/Cargo interface and
four existing provider pins are unchanged. PCI's configuration/effect faces
share one task; IOMMU authenticates independently. BARs and DMA memory continue
to be supplied dynamically. Missing platform hardware remains absent.

Validation used nightly-2026-10-09 and QEMU 10.0.13. `vanllia check` found
18 Units and 33 Cargo bins. The selected probe's declaration/initial-instance
order was System → Hub → IOMMU → PCI → VirtIO block → discovery probe.
The production closure omits only the transient probe.

```sh
vanllia check
vanllia plan --unit test.virtio-blk-discovery
vanllia build --offline --unit test.virtio-blk-discovery
vanllia build --offline
cargo clippy --manifest-path tests/product-lifecycle/Cargo.toml \
  --test drivers --target x86_64-unknown-linux-gnu --locked --offline -- -D warnings
```

The initial offline attempts needed uncached historical Terminal and accepted
IOMMU Git sources. Locked online builds populated those caches; the subsequent
offline builds passed. The final probe online/offline initrd hashes match:
`8b60ad97a06d922612678bddedd74fd11c7be12567a1427f8600392951a7eb41`
(1982464 bytes). The unchanged default product also builds offline with its
17 executables. Root `vanllia.lock` and provider source Cargo locks are unchanged;
the artifact receipts retain the actual Cargo overlay locks and all source hashes.

The new ignored `drivers` host target reuses the provider-owned
`test.virtio-blk-discovery` probe. It snapshots the exact artifact, validates an
explicit OpenSBI file/hash/length, enables `virt,iommu-sys=on`, and attaches
`virtio-blk-pci,disable-legacy=on,iommu_platform=on` with a read-only 4 MiB zero
disk. Firmware is not committed. Reproduction is in the
[host test README](../tests/product-lifecycle/README.md).

The final target passed. The authenticated probe observed the registered
`virtio-blk-pci-1`; serial also records IOMMU domain establishment for task 7,
requester 8. The host required all four drivers Ready, exactly five instances,
probe native Reap/zero, acknowledged retirement of every instance, System
completion and natural VM exit zero. No timeout or host-forced stop supplied a
passing result. The host target also passed strict Clippy and independent review.

Kernel SHA-256:
`daef24dfcc03ac138f9067e98e5994b2390b561577fb0f56b96a417563ba6d6b`.
OpenSBI SHA-256:
`8f34d231fde9554e1a3c511bb1efac489da310ab0462504c512994601e12d213`
(273472 bytes). Raw UART serial SHA-256:
`cd7248ad3336918c52128410c7602b19194a72f11d9d0a9ba08b2860c91557f4`.
The [result summary](driver-intake-results.json) preserves the final identities
and verdict. Unnormalized native serial remains in the private Library handoff;
its bytes and original checkpoint are excluded from this public Git history.

This validates QEMU discovery, observed domain establishment, Hub enrollment and
native lifecycle cleanup. Block requests, queue DMA, data integrity, reset or
recovery, DMA isolation, physical hardware and SMP are not verified. The current
resident coordinator still needs an execution consumer for real block I/O.

The System fix is published as [System #3](https://github.com/shuyra-arche/system/pull/3)
and the aggregate as [Driver #7](https://github.com/shuyra-arche/driver/pull/7),
both Drafts. The user authorized publication of the migration code and Draft
PRs without merging. Automatic approval review separately withheld the raw
UART log because that grant did not clearly cover runtime-trace disclosure.
This public candidate starts directly at main and includes neither the log
nor the earlier private checkpoint's ancestry. Executable code, source pins
and the host verdict logic match the tested checkpoint. The complete private
evidence remains available in the Library handoff. No merge or VirtIO Ready
transition is performed by this delivery.
