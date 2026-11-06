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
