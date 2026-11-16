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
- `components/`: pinned Kernel, System and service repositories.
- `tests/product-lifecycle/`: explicit product, Login and driver QEMU acceptance.
- `tests/platform/qemu/`: [trusted QEMU boot image](tests/platform/qemu/README.md) for Kernel IOMMU acceptance.
- `tests/syscall-cost/`: optional paired syscall benchmark.

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
is not a runtime verdict. [Product lifecycle tests](tests/product-lifecycle/README.md)
document the additional explicit QEMU gates and their limits.

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
The build library, platform images and native memory API/Client/Server
composition pin Kernel `f2ea64a9e2573e0796a194f3149dabe9a7f12a92`, the
coordinated User startup candidate for Kernel #160/#206/#188.
It retains the native Nest creation repair from [Kernel #175](https://github.com/shuyra-arche/kernel/pull/175)
and managed DTB/initrd sources from #180, adding bounded all-Hart CPU completion.
Executable SMP requires explicit all-Hart qualification; this pin does not qualify a platform or
claim complete System acceptance.
ABI/Ledger are 14/14 and Capsule format 2. Merge the Kernel provider before
this consumer change; this pin is not a new release. All Kernel Git dependencies use that full `rev`, including
the `components/kernel` gitlink; `[sources.kernel].path` remains unchanged.
Five Cargo lockfiles cover the root, RV64 platform, QEMU platform, hardware
fixture and native memory workspaces. `cargo run --locked --manifest-path tests/runner/Cargo.toml --target x86_64-unknown-linux-gnu -- versions --fixture .`
checks their coordinated source identity. No Cargo feature selects the ABI.
The original [26.11.14-preview-Gamma](https://github.com/shuyra-arche/kernel/releases/tag/26.11.14-preview-Gamma)
tag remains at `9a35dd5420474ebe75a6b5f3e4589a398745e32c`. Its native debug
acceptance exposed a Unit commit-lock assertion during Nest creation. The
product deliberately selects the later exact repair commit; it does not
relabel or modify Gamma, and no new release tag is claimed for this pin.
The build library writes capsule v2 with ABI 14;
the Kernel parser validates generated
capsules and retained artifacts before execution. Older ABI capsules require a
rebuild even when receipt hashes match. The historical build-library declaration
workflow still carries legacy dependencies and is not evidence of a clean
assembled System dependency graph.

The separate `tests/device/guest` workspace retains Kernel
`5e6ea8ff5f48f5ce17c88d98cbedcc7a0ffe3a9e`; its ignored device runner
requires migration before use with the current packager. The independent
`tests/product-lifecycle` lockfile still records Kernel
`a9986c82127cf25183bf5c30e7d42e4a9376b734`; it also needs a coordinated
refresh and validation. The optional `tests/syscall-cost` guest and legacy
System/Driver providers remain outside this Gamma acceptance. These paths
are not selected by the platform or native memory workspaces, and root host
tests do not execute their guests. Full System/service migration belongs to
[System #10](https://github.com/shuyra-arche/system/issues/10),
[System #13](https://github.com/shuyra-arche/system/issues/13) and
[Hub #3](https://github.com/shuyra-arche/driver-hub/issues/3).

Gamma's release notes and the historical timings in the
[native memory guide](tests/memory/README.md) are not execution evidence for
this exact Vanllia composition. New Host, RV64 and QEMU results, exact source
revisions and artifact identities are recorded separately in the
[provider acceptance PR #38](https://github.com/shuyra-arche/vanllia/pull/38).
Hardware, SMP and the complete assembled System are separate acceptance gates.

The ABI 13 migration selects System's published candidate
`470e97f38aa904c3702b665c0d225809b9f0d16e` in the product gitlink and native
System/Twine composition. Toolchain's coordinated consumer is [Toolchain #19](https://github.com/shuyra-arche/toolchain/pull/19); the
production Loader -> real std-waits execution/reclamation/retirement is covered
by the independent composition job. Full product artifact distribution remains
a separate #19/#39 acceptance gate. Existing historical Driver/product/device fixtures retain their explicitly
recorded older dependencies and are not evidence for this candidate.
