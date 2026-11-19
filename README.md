# Vanllia

Vanllia assembles Shuyra's operating system from pinned provider repositories.
The root `vanllia` CLI discovers provider-owned Unit declarations, builds
their Cargo binaries and packages them for System. Clusters organize source and
build work; Units describe runtime policy.

## Source map

- `vanllia.toml`, `vanllia.lock`, `.gitmodules`: product defaults and source selection.
- `src/`: Vanllia CLI; [CLI guide](docs/cli.md).
- `crates/build/`: `vanllia-build` library, its tests and workflow examples.
- `tests/cli/`: CLI integration tests.
- `product/tests/`: selected Shuyra **product-owned** Native/negative-boot
  composition fixtures and their Host runner (not generic CLI tests).
- `components/`: pinned Kernel, System and service repositories.
- `product/tests/lifecycle/`: historical Login/Driver/product acceptance probes;
  their old independent lock has **not** been qualified for the current product.
- `tests/platform/qemu/`: [retained QEMU platform fixture](tests/platform/qemu/README.md)
  with PCI Host checks and RV64 image builds, not a current Kernel IOMMU acceptance gate.

## Build and run

Use a Linux host with Git, Rustup/Cargo, the RV64 target, `rust-src` and LLVM's
`llvm-objcopy` on PATH. QEMU system RISC-V and a separately supplied, identified
OpenSBI image are needed for execution. Select a compatible nightly explicitly
with Rustup; this repository does not impose a `rust-toolchain.toml` override.
The CLI declares its minimum Rust version in its manifest. Guest builds require
nightly Cargo's `resolver.lockfile-path` support. Install `rust-src`, the RV64
target and LLVM tools for the toolchain you select, and verify the full product
with that toolchain before relying on its artifacts.

```sh
git submodule update --init --recursive
cargo install --path . --locked --target x86_64-unknown-linux-gnu
vanllia fetch
vanllia check
vanllia plan
vanllia build
```

The runner uses four Harts by default (explicitly configurable via `[runner].cpus`); non-benchmark CI executes its selected contracts on one Hart and does not exercise SMP qualification. Benchmark Hart counts are selected by the benchmark owner.

The default selection is Login and Daemon. System is always included. Selected
Units and their resources are packaged; requested Units such as Shell need not
start at boot. Repeated `--unit` options select a smaller dependency closure:

```sh
vanllia plan --unit test.pipe-roundtrip
vanllia build --unit test.pipe-roundtrip
vanllia test -- --list
```

Before executing an artifact, configure `[boot.firmware]` with `path` (or HTTPS
`url`), `sha256` and `bytes`, then rebuild. See [firmware inputs](docs/firmware.md).

```sh
vanllia run --artifact /absolute/path/to/.vanllia/artifacts/BUILD
vanllia test -- pipe.roundtrip
```

Artifacts and native receipts live under `.vanllia/`. A successful build alone
is not a runtime verdict. [Historical product lifecycle probes](product/tests/lifecycle/README.md)
are ignored and have not been qualified against the current product lock;
they are **not** passing release acceptance.

## Development and checks

```sh
vanllia workspace --source system=/work/system
vanllia workspace --manifest /work/system/server/Cargo.toml
vanllia workspace --reset-source system
```

Local choices live in ignored `.vanllia/overrides.toml`. Use `vanllia fetch --update`
only when intentionally refreshing source locks. CLI configuration and test
semantics are documented in [the CLI guide](docs/cli.md).

The root Cargo workspace builds host tools. Guest providers remain independent
workspaces; the CLI supplies their explicit RV64 target. `.cargo/config.toml`
retains RV64 rustflags without making RV64 the default for host commands.

```sh
cargo test --workspace --locked --target x86_64-unknown-linux-gnu
cargo clippy --workspace --all-targets --locked --target x86_64-unknown-linux-gnu -- -D warnings
```

The library retains its [workflow caller contract](docs/workflow-contract.md)
and [source attribution](PROVENANCE.md). Its examples are historical library
fixtures, not product defaults. See [migration notes](docs/migration.md) for
the source mapping and retired legacy facilities. Firmware, hardware and SMP
acceptance remain separate from host tests.

[Contributing and organization maintenance rules](CONTRIBUTING.md).

The normal `shuyra` image is built from [`platform/riscv`](platform/riscv/README.md).
Kernel supplies a library and normalized boot contracts; board firmware decoding,
load address and controller wiring belong to this integration package. The EDU
image under `tests/platform/qemu` reuses the production boot path.

The CLI consumes the kernel-independent VNI1 bundle from Unit at
`a0e8138275dc277f7d202fedb74cf9fa88aee368` with default features disabled.
The draft ABI 14 qualification candidate pins Kernel `44d9593d89c4823a7599eb3ee3683bd7a5cee9b5` across
the build library, RV64 platform, QEMU fixture, Native memory product Guest,
Cargo locks, and `components/kernel` gitlink. This immutable Kernel revision contains the merged #301 Journal fix, #302
Nest/CPU-retirement correction, and #321 Kernel-owned ABI 14 CPU target.
Kernel main has accepted these fixes; Vanllia main still selects the published
ABI 13 product until the separate consumer migration is accepted.

The candidate Native ABI/Ledger contract is 14/14; Capsule format remains 2.
Run `cargo run --locked --manifest-path product/tests/runner/Cargo.toml
--target x86_64-unknown-linux-gnu -- versions --fixture .` to check exact
source identity. Older ABI capsules must be rebuilt, not accepted through a
compatibility wrapper. The coordinated Native candidate is tested separately in the draft
[Vanllia ABI 14 migration PR](https://github.com/shuyra-arche/vanllia/pull/50);
its candidate checks do not establish System/Twine, multi-Hart native,
hardware IOMMU, or full product acceptance.

The original
[26.11.14-preview-Gamma](https://github.com/shuyra-arche/kernel/releases/tag/26.11.14-preview-Gamma)
tag remains unchanged. Its historical results cannot certify this candidate.

The old `tests/device` guest and experimental `tests/syscall-cost`
benchmark were retired because their historical provider graphs cannot
qualify the current ABI 13 product; their complete sources and coverage gaps
are preserved in the [migration record](docs/migration.md). The independent
`product/tests/lifecycle` lockfile still records Kernel
`a9986c82127cf25183bf5c30e7d42e4a9376b734` and requires a coordinated
product-owner refresh and validation. Its ignored scenarios are not selected
by ordinary Host, Platform or Native CI and are **not current-product acceptance**.
Legacy System/Driver providers also remain outside this product gate. Full System/service migration belongs to
[System #10](https://github.com/shuyra-arche/system/issues/10),
[System #13](https://github.com/shuyra-arche/system/issues/13) and
[Hub #3](https://github.com/shuyra-arche/driver-hub/issues/3).

Gamma's release notes and the historical timings in the
[native memory guide](product/tests/memory/README.md) are not execution evidence for
this exact Vanllia composition. New Host, RV64 and QEMU results, exact source
revisions and artifact identities are recorded separately in the
[provider acceptance PR #38](https://github.com/shuyra-arche/vanllia/pull/38).
Hardware, SMP and the complete assembled System are separate acceptance gates.

The ABI 13 migration selects System's published candidate
`470e97f38aa904c3702b665c0d225809b9f0d16e` in the product gitlink.
System now owns its own native Runtime/Supply/RPC contracts; Vanllia does not
keep a second System/Twine test runner. Toolchain's coordinated consumer is [Toolchain #19](https://github.com/shuyra-arche/toolchain/pull/19); the
production Loader -> real std-waits execution/reclamation/retirement is covered
by the independent composition job. Full product artifact distribution remains
a separate #19/#39 acceptance gate. Existing historical Driver/product/device fixtures retain their explicitly
recorded older dependencies and are not evidence for this candidate.
