# Vanllia

Vanllia combines Shuyra providers through the independent `vanllia` CLI. The
root owns `vanllia.toml`, source pins and product defaults. Provider repositories
own their Unit declarations and Cargo bins. Clusters organize repositories;
Units describe runtime policy. No central Unit/image table is required.

```sh
git submodule update --init --recursive
cargo install --path components/image-builder/cli --locked --target x86_64-unknown-linux-gnu
vanllia fetch
vanllia check
vanllia plan
vanllia build
```

Use the toolchain supported by the selected CLI and LLVM `llvm-objcopy` on PATH.
Validation uses nightly-2026-10-09; historical provider toolchain files are
unchanged. Cargo supplies dependency resolution and compilation inside provider
workspaces. The root has no Cargo product adapter, bootstrap or composition crate.

The default roots are Login and Daemon. Necessary Units and Shell's native Cargo
image resources are discovered from provider TOML. The host builds and packages
all selected Unit templates in initrd. System is the always-included core boot
Unit; Operator/Identity bootstrap are its internal threads. Shell is requested
and can be instantiated at runtime. Packaging does not imply startup creation.

Driver also includes optional PCI, IOMMU and VirtIO block providers. Select
`virtio-blk` for their minimal resident closure, or use the provider's
`test.virtio-blk-discovery` Unit with the opt-in host QEMU smoke. See
[driver intake validation](docs/driver-intake-validation.md) for the exact
source pins, hardware inputs, native verdict and remaining I/O limits.

```sh
vanllia plan --unit test.pipe-roundtrip
vanllia build --unit test.pipe-roundtrip
vanllia test -- --list
```

The new Pipe case is written in Rust and uses native exit and resource retirement
receipts. Kernel event logging is removed. Old scenarios and their tests are not
migrated; retained composition test sources and historical docs are archival
evidence rather than current product entry points.

```sh
vanllia workspace --source system=/work/system
vanllia workspace --manifest /work/system/server/Cargo.toml
vanllia workspace --reset-source system
```

Local choices live in ignored `.vanllia/overrides.toml`; the editor configuration
and build use the same Cargo overlay. The CLI prepares missing packages and
locks real commits; refresh intentionally with `vanllia fetch --update`.

Build receipts identify exact initrd/kernel bytes, debug images, Cargo locks,
toolchain and source hashes under `.vanllia/artifacts/`. To run a saved artifact,
configure a specific firmware input first, then rebuild the artifact to include
it. Add `[boot.firmware]` with the installed OpenSBI `path`, `sha256` and `bytes`.
Obtain the digest and length with `sha256sum` and `wc -c`. The CLI checks this
input; it does not redistribute a firmware binary or silently choose another.

```sh
vanllia run --artifact /absolute/path/to/.vanllia/artifacts/BUILD
vanllia test -- pipe.roundtrip
```

These last two commands execute managed QEMU with headless serial evidence.
The minimal Pipe case passed 1/2/4-core QEMU with native exit and reclaim checks;
the unchanged full product also passed startup and terminal EOF cleanup at each
CPU count. A bounded paired syscall benchmark measured event-log removal.
See [docs/dynamic-build-validation.md](docs/dynamic-build-validation.md)
for current evidence. Earlier `cargo image`, Nushell build/runner entries and
static Deployment files are retired.
