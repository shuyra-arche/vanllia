# Build tools consolidation

The host tools are imported from image-builder commit
`82f81b121a9c8f0590ceceb51c77fa0369355217` into Vanllia. The former
`components/image-builder` gitlink is removed. Other provider pins and
`vanllia.lock` are unchanged. The root Cargo lock is copied without dependency
updates from that image-builder revision.

| Former image-builder path | Vanllia path |
| --- | --- |
| `cli/src/` | `src/` |
| `cli/tests/` | `tests/cli/` (explicit Cargo test targets) |
| `src/`, `tests/`, `examples/` | `crates/build/` |
| `cli/README.md` | `docs/cli.md` |
| `docs/workflow-contract.md` | `docs/workflow-contract.md` |

Install with `cargo install --path . --locked --target x86_64-unknown-linux-gnu`.
Library users retain package `vanllia-build` and crate `vanllia_build`, now at
`crates/build`. The lifecycle tests use the new local paths. The CLI's discovery,
public APIs, publication receipts and runtime policy are unchanged. The fixture
workspace remains independent; its relative paths and the examples' `library`
and `{tool}` paths still resolve within `crates/build`.

Host commands now default to the host target. RV64 flags remain under the
target-specific Cargo table, and guest builds continue to pass an explicit
target. Kernel, System and service repositories remain independent workspaces.

The optional `firmware/` build recipe is removed. Identified external firmware
inputs, hash/byte verification and QEMU support remain; see [firmware inputs](firmware.md).

## Retired legacy facilities

The legacy Python test suite, its fixtures/helpers and assertion/coverage
baselines, `acceptance/`, `composition/`, legacy evidence scripts and composition
input generator are removed. This is an explicit retirement of that coverage;
it does not claim equivalent coverage from the current Rust tests. The removed
files remain available in Git history at `4e030986c2cf47d7bdb88ec37576323b75d8540a`.
The original source/blob attribution is preserved unchanged in
[`provenance/legacy-source-files.json`](provenance/legacy-source-files.json);
its path entries describe the retired files.

Current CLI/build-library tests, product lifecycle gates and the syscall-cost
benchmark remain. The imported build-library examples (including their Python
runner adapters and fixed external source recipes) remain as library fixtures.
They do not depend on the removed in-repository acceptance machinery.

The root `rust-toolchain.toml` is removed. Developers select their own compatible
nightly and install the required target/components explicitly. The fixed
toolchain in historical library example recipes belongs to those examples and
does not set the repository's default toolchain.

## Retired Kernel Device test fixture (2026-11-18)

The old root Cargo `device` test was always ignored during normal Host CI
and depended on a Guest pinned to Kernel
`5e6ea8ff5f48f5ce17c88d98cbedcc7a0ffe3a9e`. Its runner constructed
the boot image with the current `vanllia-build` encoder, which has a different
Kernel ABI. It could not qualify the currently selected ABI 13 product.

The legacy `tests/device/` suite and root `[[test]] device` entry have been
retired as a **historical, non-executable contract**, not silently reclassified
as a passing test. The complete seven-file source and its exact Guest lock
remain recoverable in Git at
[`73b939d764711b321da12c1b7aea022998da51d6`](https://github.com/shuyra-arche/vanllia/tree/73b939d764711b321da12c1b7aea022998da51d6/tests/device).

That suite once targeted a U-mode Driver, restricted EDU MMIO access,
CPU/DMA exclusion, invalid IOVA fault routing, source revocation and
lifetime/retirement across Unit failure. The existing Kernel M-mode IOMMU
checks are **not equivalent** to this full User-Driver/System contract. Its
current-ABI replacement belongs to the actual Kernel/Driver consumer or
product configuration owner, not Vanllia's CLI tests. No replacement is
claimed or added here; the corresponding coverage gap remains explicit
until an owner validates the behavior at a compatible revision.

This retirement does not update Kernel ABI, any product pin or lockfile,
normal QEMU integration matrices, or `vanllia test` semantics.
