# vanllia M1 product composition

Candidate M1 only. It owns one product `Deployment`, the new `system` bootstrap and
17 explicit `ImageBuild` coordinates. Runtime and host both consume
`vanllia-composition`, so they cannot select different product scenes.

## Commands

Clone the fixed provider tree and use the original toolchain:

```sh
git clone --branch codex/daemon-intake --recurse-submodules https://github.com/shuyra-arche/vanllia.git
cd vanllia
rustup component add llvm-tools-preview --toolchain nightly-2026-09-04
export PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH"
```

From this directory (Linux x86_64 host):

```text
cargo image check
cargo image build
cargo image build product debug
```

`cargo image` runs the host adapter `vanllia-image` for
`x86_64-unknown-linux-gnu`. Defaults are `build product release`; the output is
`target/product/release/initrd.img`. The adapter parses command, scene, profile
and optional `--root`, then calls `vanllia_build::{image_inventory,startup_order,
build_with}`. It does not invoke QEMU and does not implement a build engine.

Equivalence tests against the PR55 legacy composition:

```text
cargo test --locked -p vanllia-image --target x86_64-unknown-linux-gnu --test equivalence
```

## M1 legacy edges (disclosed)

The following images still come from the legacy sqware tree at full SHA
`94e114614bec53424ac55e640fc80cf19a3e7107`, via its own
`legacy/sqware/programs/Cargo.toml` and lock:

- `login`, `uart`, `rtc`
- `shell`, `cat`, `emit`, `upper`, `fail`, `spin`, `workers`

The Daemon declaration and image now come from the immutable provider
`https://github.com/shuyra-arche/daemon.git` at full revision
`4e4ae06ee488e9f02e15d74405b989f20a5a286f`; `daemon-declaration` owns `UNIT` and
`BUILD`, and `daemon-server` builds bin `daemon` from `server/Cargo.toml` in the
fixed `components/daemon` gitlink. The selected Daemon normal
dependency graph is independent of `sqware`, `programs`, `composition` and
`system-server`. Legacy programs' aggregate lib may still compile the old daemon
code; that is not part of the selected Daemon closure. This slice adds no QEMU
or runtime acceptance claim.

The production `rtc-api` package is now pinned independently to
`https://github.com/shuyra-arche/driver-rtc.git` at full revision
`621be2b9946827be3de4245e29aa74ceb0f8cbd0`; the package name remains `rtc-api`.
That crate owns pure protocol types/constants/failure codec only. The RTC runtime
program, client and server still come from legacy sqware, as does
`shell-declaration`. The dev-only `legacy-composition` used by the host
equivalence test retains the old sqware `rtc-api` and `fixturetarget`
declarations. Published servers remain `components/{driver,session}/units/...` and
`components/pipe/server`. The new `system` image is `bootstrap/Cargo.toml`; it uses
published `system-server` at `9001af65...` and `system-support` at `5a792ff2...`,
and is never the legacy system binary. M1 is therefore not independent of sqware.

## Boundaries

The root Cargo lock owns the three local packages; provider and legacy workspaces
retain their existing locks. Cargo compiles all images through the existing
`vanllia-build` library. It resolves actual Cargo executable artifacts and
publishes the initrd; this repository adds no source manager or build engine.

The fixed Kernel can be built separately:

```text
cargo build --locked --manifest-path components/kernel/Cargo.toml --release --bin shuyra --features semihosting --target-dir target/kernel
```

Boot transport, firmware and the original acceptance scenarios still reside in
the disclosed legacy repository (`scripts/boot.nu`, `scripts/qemu-args.nu`,
`SBI.bin` and tests). They are additional sqware dependencies. An acceptance
driver must select this repository's initrd and new bootstrap rather than
rebuilding the legacy product image.

M2 is not complete: the ten production images above, `shell-declaration`, and
firmware/boot, scenarios and tests must leave sqware before independence can be
claimed. TOML/UnitFile redesign is undecided. The old PR1 single SMP8 evidence is
prior-tree only; this slice has no QEMU and claims no runtime acceptance.
