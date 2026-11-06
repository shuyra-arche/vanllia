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
