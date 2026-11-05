# vanllia M1 product composition

It owns one product `Deployment`, the new `system` bootstrap and
17 explicit `ImageBuild` coordinates. Runtime and host both consume
`vanllia-composition`, so they cannot select different product scenes.

## Commands

From a checkout of the reviewed revision, initialize the fixed provider tree and
use the original toolchain:

```sh
git submodule update --init --recursive -- components/kernel components/system components/image-builder components/driver components/session components/pipe components/daemon components/shell
rustup component add llvm-tools-preview --toolchain nightly-2026-09-04
export PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH"
```

This initializes the current provider trees. The frozen `legacy/sqware`
gitlink is no longer registered, so unrestricted recursive initialization also
fetches only current providers. An existing local checkout is preserved and
ignored at that exact path; normal builds and schema-2 evidence do not require
its content or Git objects. Missing or changed current providers still fail
source verification. Retained schema-1 records keep their original meaning.

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

For the combined Kernel + product build, install Nushell **0.107.0** and use
the Vanllia-owned entry:

```sh
nu scripts/build.nu --plan           # inspect commands; no build or QEMU
nu scripts/build.nu release          # build both; emit a JSON artifact record
nu scripts/build.nu debug
```

The combined release entry has been built locally; see the exact
[build results and firmware license findings](docs/integration-build-validation.md).

The record contains `scene`, `profile`, `initrd` and `kernel`. Paths come from
the image adapter's `ok …` output and Cargo's `compiler-artifact` JSON, including
custom output paths; the script does not guess an old initrd location. Command
diagnostics go to stderr. The build uses the pinned Kernel, the explicit RV64
target and the Kernel `semihosting` feature, and never starts QEMU.

Boot and acceptance use Python 3 (tested with 3.12), Bash, GNU coreutils, Nushell
0.107.0 and `qemu-system-riscv64` (the source transport was used with QEMU 10.x).
For the Kernel test runner, install the precise host package, rather than the
unrelated `cargo-qtest` package:

```sh
cargo install cargo-qemu-test --version 0.3.2 --locked --target x86_64-unknown-linux-gnu
```

QEMU uses its bundled OpenSBI through `-bios default`; `SHUYRA_SBI` may be unset,
empty or `default`. A custom firmware path is validated before launch. Install
QEMU's matching firmware data package; missing firmware is a startup error.
See [firmware setup and provenance](docs/firmware.md).
After runtime authorization, a Nushell session can consume the build record:

```nu
let artifacts = (nu scripts/build.nu release | from json)
$env.SHUYRA_INITRD = $artifacts.initrd
# Optional override: $env.SHUYRA_SBI = ("/absolute/path/to/fw_jump.bin" | path expand)
$env.QEMU_SEMI = "1"
nu scripts/runner.nu $artifacts.kernel
# Separate acceptance run (starts QEMU and rebuilds the selected product image):
nu scripts/qtest.nu --package kernel --scene product
```

`runner.nu` preserves the original interactive observation/archive behavior; its
normal return is not an acceptance verdict. `qtest.nu` owns the acceptance
verdict. Only `product` has a migrated image deployment; other scene names fail
early. Host-only tests and the retained scenario inventory are documented in
[acceptance/README.md](acceptance/README.md). No runtime result for this candidate
is claimed.

Independent contracts now retain the Daemon, Login, UART, RTC and Shell
declaration fields, registry slots and ownership checks. The
[third migration map](docs/test-migration-batch3.md) records the assertion
successors and removal of the last legacy composition dev dependency. Product
plan, complete image coordinates, owner routes and typed Shell catalogue/resource
bindings remain covered by their existing composition contracts.

Product declaration and plan contracts live with composition and use frozen
literal expectations. Run the current host contracts:

```text
cargo test --locked --offline -p vanllia-composition -p vanllia-image --tests --target x86_64-unknown-linux-gnu
```

See [the contract coverage and provenance record](docs/product-contract-tests.md)
for the standalone #8 baseline. Those five contract sources are unchanged;
their coordinate-order case checks names/order, while the additional image
contracts now freeze all descriptor fields and actual provider routes.

The [first namespace policy test migration](docs/test-migration-batch1.md)
preserves three old host cases and adds a check of the actual product registry.
That first checkpoint retained all old comparisons; the subsequent migration
retires only comparisons with mapped independent successors. The current
combined Rust host gate has 19 cases.

## Provider and legacy boundaries (disclosed)

All 17 selected images have explicit product coordinates. Zero selected images
use `legacy/sqware/programs/Cargo.toml`, and the production normal dependency
graph has zero sqware edges. The root locked graph, including host development
dependencies, now has zero sqware packages. The old gitlink is removed from
the current tree; its frozen SHA and migration records remain in history and
documentation. Existing local source data is retained, and old tests/scenes
are not migration prerequisites. See [the current scope](docs/current-migration-scope.md).
This does not establish runtime acceptance or archive readiness.

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
identity updated. Account, Hub and Router pins remain unchanged. The new literal
contracts retain the old Login/UART/RTC declaration expectations without loading
the legacy composition crate.

Shell now comes from `https://github.com/shuyra-arche/shell.git` at full revision
`5d84650d37df7523d0ac59fc563e2fc7dc4025ba`. `shell-declaration` owns the
`PROGRAM`, catalogue and `BUILD`, while `shell-images-declaration` owns the
ordered `BUILDS` aggregate for exactly seven images: `shell`, `cat`, `emit`,
`upper`, `fail`, `spin` and `workers`. The composition only prefixes each
provider-relative manifest with `components/shell/`. The provider manifests are
`server/entry/Cargo.toml`, `cat/command/Cargo.toml` and
`commands/Cargo.toml`. `shell` builds package `shell-entry` bin `prog-shell`;
`cat` builds package `cat` bin `prog-cat`; the five command images build package
`shell-command-images` with bins matching their image names. All seven runtimes
use in-provider SDK paths, so separate immutable Cargo workspaces may carry
normal local Cargo IDs without creating mixed Git SDK identities. The old Shell
client pin `b910582fc9dc0fd732f279d94f2cfca89f4c8c8f` is historical: the live
Shell server, client, API, Lisp, commands and Cat use the pinned provider tree.
The superseded sqware `shell-declaration` at `94e1146...` and the direct
`shell-command-images-declaration` at `effeb09...` are not selected.

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
build from the Driver provider. The former dev-only legacy composition and its
old sqware RTC API/fixturetarget and superseded Driver/Session/Terminal package
identities have been removed from the root lock. Published servers remain
`components/{driver,session}/units/...` and `components/pipe/server`. The new
`system` image is `bootstrap/Cargo.toml`; it uses published `system-server` at
`9001af65...` and `system-support` at `5a792ff2...`, and is never the legacy
system binary.

## Boundaries

The root Cargo lock owns the three local packages; provider and legacy workspaces
retain their existing locks. Cargo compiles all images through the existing
`vanllia-build` library. It resolves actual Cargo executable artifacts and
publishes the initrd; this repository adds no source manager or build engine.

The fixed Kernel can be built separately:

```text
cargo build --locked --manifest-path components/kernel/Cargo.toml --release --bin shuyra --features semihosting --target-dir target/kernel
```

Boot transport and Python acceptance checkers now reside in this repository's
`scripts/` and the `acceptance/` runtime package. Host tests live separately in
`tests/` and run with `python3 -B -m unittest discover -s tests -t .`. They do not
read the old checkout. See [test ownership](docs/acceptance-ownership.md). Firmware
defaults to QEMU's bundled OpenSBI; the unverified old `SBI.bin` is not copied. The
new qtest entry builds this repository's product initrd and new bootstrap.
Non-product scene deployments and their guest fixture programs still need
provider migration before they can be enabled here.

Current scope is the product code, build and required input verification.
Wholesale migration of the old tests and 35 guest scenes has stopped; planned
destructive changes will need tests for the resulting behavior. Existing tests,
source and completed checkpoints remain available as evidence.
Firmware provenance and new runtime evidence also remain open. TOML/UnitFile
redesign is undecided. The old PR1
single SMP8 evidence is prior-tree only; this slice has no QEMU and claims no
runtime acceptance.

See the [entry migration boundary](docs/boot-entry-migration.md) for exact source
heads and the remaining archive gates. This candidate does not establish
whole-repository independence or runtime acceptance.
The [historical fixture and scene inventory](docs/legacy-migration-order.md)
records previous dependencies; it is not a migration checklist or acceptance gate.
