# vanllia M1 product composition

Candidate M1 only. It owns one product `Deployment`, the new `system` bootstrap and
17 explicit `ImageBuild` coordinates. Runtime and host both consume
`vanllia-composition`, so they cannot select different product scenes.

## Commands

Clone the fixed provider tree and use the original toolchain:

```sh
git clone --branch codex/login-uart-rtc-intake --recurse-submodules https://github.com/shuyra-arche/vanllia.git
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

Product declaration and plan contracts live with composition and use frozen
literal expectations. Run them alongside the retained equivalence tests:

```text
cargo test --locked --offline -p vanllia-composition -p vanllia-image --tests --target x86_64-unknown-linux-gnu
```

See [the coverage and provenance record](docs/product-contract-tests.md). All five
original equivalence tests remain unchanged; this slice removes no legacy edge.

The same suite checks the five migrated command builds against the public
descriptor, compares the selected Login/UART/RTC owner builds and observable
`UnitFile` fields with the frozen legacy declarations, and verifies the frozen
Shell catalogue ports and bindings.

## M1 legacy edges (disclosed)

The following images still come from the legacy sqware tree at full SHA
`94e114614bec53424ac55e640fc80cf19a3e7107`, via its own
`legacy/sqware/programs/Cargo.toml` and lock:

- `shell`
- `cat`

The `login` image and `PROGRAM` now come from the immutable Session provider
`https://github.com/shuyra-arche/session.git` at full revision
`87abc22d08ffe311b7e43d915240d318642d510c`; `session-cluster` reexports
`login::{PROGRAM,BUILD}`, and `login-server` builds bin `login` from
`components/session/units/login/server/Cargo.toml` in the fixed
`components/session` gitlink. The Login leaf owner is
`https://github.com/shuyra-arche/session-login.git` at full revision
`ce8e9007817d7494e05c057e23a7ba907a5d5ba8`; Session `87abc22...` selects and
reexports it, so the aggregate revision is distinct from the leaf owner
revision. The `uart` and `rtc` images and `UNIT`/`BUILD` declarations now come
from the immutable Driver provider at full revision
`bb3427e179e87b640e0d51115a44544fae231252`; `driver-cluster` reexports
`uart` and `rtc`, with `uart-server`/`rtc-server` building bins `uart`/`rtc`
from `components/driver/units/{uart,rtc}/server/Cargo.toml`. The Driver RTC
declaration uses RTC API at full revision
`f14cca51af02061350b2d1aa9f00f95c6dc06e15`; the composition no longer carries
a direct `rtc-api` dependency at the superseded `621be2b9...` revision. The
UART declaration uses the canonical driver-uart package at full revision
`298bc8809f297f46bc7592aebc96540b7b492efc`. The Terminal gitlink/declaration
changed from `f07a0954f65472e9a16e878e52c5f1d34146bc64` to
`d120e3ddf4b4a6c00685f32310897622d3440dd5` to align UART `298...`; its runtime
source and declaration bytes are unchanged, with only provider intake/manifest
identity updated. Account, Hub and Router pins remain unchanged, and the frozen
dev-only legacy composition retains the old Login/UART/RTC declarations as
source fixtures.

The five Shell command images `emit`, `upper`, `fail`, `spin` and `workers`
now build from `components/shell/commands/Cargo.toml` in the public Shell
gitlink at full revision `effeb09c170f0259c3d79687754b02aa41f615fe`. Their
build coordinates come from the `shell-command-images-declaration` package in
`components/shell/commands/declaration`; the composition only prefixes its
provider-relative `commands/Cargo.toml` manifest with `components/shell/`.
The Shell catalogue, resource names, ports and bindings remain owned by the
unchanged sqware `shell-declaration`; these five images are resources, not new
`UnitFile`s. The command runtime continues to pin the public Shell client at
`https://github.com/shuyra-arche/shell.git` revision
`b910582fc9dc0fd732f279d94f2cfca89f4c8c8f`; its nearby `client/` tree is not
substituted. The legacy programs aggregate may still contain the old command
code, but the five selected command images now build from the independent
`components/shell/commands/Cargo.toml` closure. This slice does not intake the
Shell server, Cat or Lisp.

The Daemon declaration and image now come from the immutable provider
`https://github.com/shuyra-arche/daemon.git` at full revision
`4e4ae06ee488e9f02e15d74405b989f20a5a286f`; `daemon-declaration` owns `UNIT` and
`BUILD`, and `daemon-server` builds bin `daemon` from `server/Cargo.toml` in the
fixed `components/daemon` gitlink. The selected Daemon normal
dependency graph is independent of `sqware`, `programs`, `composition` and
`system-server`. Legacy programs' aggregate lib may still compile the old daemon
code; that is not part of the selected Daemon closure. This slice adds no QEMU
or runtime acceptance claim.

The production RTC API now resolves through the Driver provider's RTC
declaration at full revision `f14cca51af02061350b2d1aa9f00f95c6dc06e15`; the
composition no longer carries a second direct `rtc-api` dependency at
`621be2b9946827be3de4245e29aa74ceb0f8cbd0`. That crate owns pure protocol
types/constants/failure codec only. The selected RTC server and image also
build from the Driver provider, while `shell-declaration` still comes from
legacy sqware. The dev-only `legacy-composition` used by the host equivalence
test retains the old sqware `rtc-api`, `fixturetarget`, Driver/Session and
Login/UART/RTC declarations. Published servers remain
`components/{driver,session}/units/...` and `components/pipe/server`. The new
`system` image is `bootstrap/Cargo.toml`; it uses published `system-server` at
`9001af65...` and `system-support` at `5a792ff2...`, and is never the legacy
system binary. M1 is therefore not independent of sqware.

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

M2 is not complete: the two remaining legacy images `shell` and `cat`,
`shell-declaration`, the legacy aggregate and host fixtures, and the
firmware/boot/scenarios/tests dependencies must leave sqware before
independence can be claimed. TOML/UnitFile redesign is undecided. The old PR1
single SMP8 evidence is prior-tree only; this slice has no QEMU and claims no
runtime acceptance.
