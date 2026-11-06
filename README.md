# Vanllia

Vanllia assembles Shuyra's operating system from pinned provider repositories.
The independent `vanllia` CLI discovers provider-owned Unit declarations, builds
their Cargo binaries and packages them for System. Clusters organize source and
build work; Units describe runtime policy.

## Source map

- `vanllia.toml`, `vanllia.lock`, `.gitmodules`: product defaults and source selection.
- `components/`: pinned Kernel, System, build CLI and service repositories.
- `tests/product-lifecycle/`: explicit product, Login and driver QEMU acceptance.
- `tests/syscall-cost/`: optional paired syscall benchmark.
- `acceptance/`, `composition/`, Python tests and evidence scripts: retained older
  test machinery; these do not define the current CLI's product or test catalog.

## Build and run

Use a Linux host with Git, Rustup/Cargo, the RV64 target, `rust-src` and LLVM's
`llvm-objcopy` on PATH. QEMU system RISC-V and a separately supplied, identified
OpenSBI image are needed for execution. `rust-toolchain.toml` pins
`nightly-2026-09-04`; the CLI also declares a minimum Rust version in its manifest.
The pinned environment still needs a fresh full-product verification; earlier
runs with another nightly are not evidence that this pin works.

```sh
git submodule update --init --recursive
cargo install --path components/image-builder/cli --locked --target x86_64-unknown-linux-gnu
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
semantics are documented in the selected `components/image-builder/cli/README.md`.

The retained Python tests include legacy entry checks whose Nu scripts have been
removed. Their complete discovery is not currently a passing product gate. The
self-contained oracle and transport tests can still be run independently; see
[acceptance ownership](acceptance/README.md). Do not suppress failures to make the
legacy suite appear current. Firmware, hardware and SMP acceptance remain
separate from host tests.

[Contributing and organization maintenance rules](CONTRIBUTING.md).
