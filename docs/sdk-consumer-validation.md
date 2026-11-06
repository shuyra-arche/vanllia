# Kernel SDK consumer validation

The product now resolves its existing consumers through Kernel's single API and
Client. The API owns ABI types, codes and wire conversion. The Client owns calls
and client state. Env, Wire and Mold keep their compatibility facades; Execution
uses Client implementations while retaining the existing startup policy. No
provider implementation, Runtime exit policy or historical test was changed in
this product integration.

## Published inputs

| Source | Selected revision |
| --- | --- |
| Kernel component gitlink | `90d43dbf3ee43fd146f487718eb39e88f2507c05` |
| Env | `f54f9a1ab3f0df3631fb80ff2bf5d0cfaecbe927` |
| Execution | `ea49192e5df8bf744e083ced23ff0908b5bab3a3` |
| Wire | `4b7619fda4bbd202e9a5c952872a84c2145c681e` |
| Mold | `1addd302789e6a1c0dea6a7adf27e975a05d5ada` |
| IOMMU | `ba78aa607f2d7cfd03381cfec9eea1ad2cc4c825` |

The SDK Git sources are pinned in `vanllia.toml`; the CLI's real
`fetch --update` generated `vanllia.lock`. Other component gitlinks and the
Unit source pin remain unchanged. IOMMU uses the canonical repository URL
`https://github.com/shuyra-arche/driver-iommu.git`. Its explicit source replaces
the nested Driver IOMMU provider during CLI discovery. The IOMMU candidate's
Server Rust bodies, linker metadata and Unit policy match its default-branch
base. Its hold dependency uses the pure Kernel API; its root selections and
Server's direct Execution pin restore the supported standalone build as well.

All six development source overrides had been reset; none was present during
this fetch, build or test. The
ignored `.vanllia/overrides.toml` has an empty `[sources]` table. Each fetched
source is a clean checkout at its public revision; the Kernel component is a
clean checkout at its selected gitlink. Build overlays refer to these fetched
sources and selected components, rather than the development sibling checkouts.
Cargo resolutions and their locks are isolated under `.vanllia/cargo`; provider
source locks were not rewritten to manufacture a product result.

## Actual release builds

Validation used nightly-2026-10-09, rustc `1.101.0-nightly (a30aa9064 2026-10-08)`,
Cargo `1.101.0-nightly (29c5daa1a 2026-10-07)` and LLVM objcopy. Every selection
used `riscv64gc-unknown-none-elf`, profile `release`, with the ordinary Kernel
feature set. All provider refs stayed frozen through build publication.

```sh
vanllia fetch --update
vanllia build
vanllia build --unit virtio-blk
# Configure the explicit local firmware fragment below for this runtime test.
vanllia test -- pipe.roundtrip --exact --nocapture
# Remove the local firmware fragment afterwards.
vanllia check
```

The final check reports 18 Units and 39 Cargo bins. The runtime test reports
one passed case, zero failed and one other case filtered out.

| Selection | Runtime images | Cargo locks | Artifact directory under `.vanllia/artifacts/` | Initrd SHA-256 |
| --- | ---: | ---: | --- | --- |
| Default Login + Daemon | 17 | 14 | `1793915666510872525-95269-0` | `62af1ad1b8659c0acaa0e3a8fb8370c786ae5e5023ff6636620c2c47fcb122a4` |
| `virtio-blk` closure | 5 | 6 | `1793915730599633273-96441-0` | `86ee368aa9cc160a8d0e4b0ab6af24bd3545f5edcd30cc8d08e4f7c8e6a6254c` |
| `test.pipe-roundtrip` | 3 | 4 | `1793915791078767552-98243-0` | `1f423543d225fe44d65f6225b07d02e39194dcc157999f3895d6bbfc6b41874f` |

Kernel is a separate boot artifact in each receipt, not one of the runtime image
counts. Its identical ELF SHA-256 is
`e1caa8f008ddfc12f327546c68896de431ab7b9b38df577e5f6020011577f4f4`
(1,794,256 bytes). The optional driver closure actually compiled System, Hub,
IOMMU, PCI and VirtIO block. Default and driver build receipts have no firmware;
only the Pipe test artifact includes the explicitly selected local firmware.

Each artifact directory retains `receipt.json`, `vanllia.lock` and its copied
`cargo-N.lock` files. All recorded image, Kernel, initrd and Cargo lock lengths
and SHA-256 values were independently checked against their files. The compact
[result ledger](sdk-consumer-results.json) records receipt hashes, copied locks
and graph owners for all three selections. These are new published build
receipts. All three were regenerated after the public IOMMU pin changed. The
initrd hashes happen to match the previous selection; the Kernel hash changed,
so the previous runtime result is not reused. The earlier development build
rejected by the source-change guard
remains an unpublished failure and supplies no passing evidence here.

## Actual dependency graphs

For every package that produced a selected image, and for Kernel itself, the
audit ran Cargo's target graph using that build's exact CLI overlay and unchanged
isolated lock:

```sh
cargo tree --manifest-path PROVIDER/Cargo.toml --config OVERLAY.toml \
  --locked --offline --target riscv64gc-unknown-none-elf \
  --edges normal,build --package PACKAGE --prefix none --format '{p}'
```

The audit covered 14 package graphs for the default product, six for the driver
closure and four for Pipe, including the packages producing Shell's multiple
command bins. Every actual graph contains exactly one `kernel-api-env` owner at
`components/kernel/api/env`. Each consumer graph contains exactly one
`kernel-client` owner at `components/kernel/client`; Kernel has none. Each graph
also has one `kernel-api-wire` and one `kernel-api-macros`, both from the selected
Kernel component. Kernel's
normal/build graph contains neither the Env facade nor Client, Execution, Wire
or Mold. Kernel, its allocator and IOMMU hold share that same pure API owner.
The entire workspace lock can contain development or unused packages; those
entries were not mistaken for active Server dependencies.

One older protocol source deliberately remains visible: VirtIO block's existing
`iommu-api` dependency still resolves from
`https://github.com/shuyra-arche/iommu.git` at
`3d210b4da522ee0c7427265ce0a0d8946e124b21`. GitHub renamed that repository to
`driver-iommu`, but the CLI matches literal origin URLs and does not normalize
that rename. The selected IOMMU Server/API/hold come from the canonical pinned
source; the VirtIO block protocol API is still the old-URL source. Its graph
nevertheless shares the single Kernel ABI and Client owners above, and the
actual driver closure builds. This result does not claim that every IOMMU
protocol package came from the new pin.

## Standalone repair and selection scope

The IOMMU baseline `d3d3a4ccf796123eb47d72f5c2751c3756f2b0af` supported raw
Server check and release build. The intermediate `90f59ef` hold migration
introduced six dual-ABI type errors; this was a regression from this work,
not an existing failure. IOMMU `ba78aa6` repairs the dependency selection while
preserving the supported standalone commands and Server behavior.

The independently reviewed
[IOMMU validation at the selected public revision](https://github.com/shuyra-arche/driver-iommu/blob/ba78aa607f2d7cfd03381cfec9eea1ad2cc4c825/CANONICAL-API-VALIDATION.md)
records raw RV64 check and release build without a Cargo `--config` overlay,
18 API/hold host tests, three Unit-document tests and active-owner checks. Its
raw release ELF is 354,208 bytes, SHA-256
`1f2118e11390b61b04c88f4775e6354c11849a0f94f2d5bf79b14e299f670da9`.
That provider evidence establishes the standalone repair; the product artifacts
above establish the separate CLI composition result.

The IOMMU workspace root uses seven exact Package ID `[replace]` selections:
Env, Wire, Mold, Kernel API, System Client, System Support and Driver Shared.
The Server directly pins Execution `ea49192e`. Its standalone System packages
come from `8eea7c9039cac0e6ada4a59c612dc6c98ef9bc45`
([System Draft #4](https://github.com/shuyra-arche/system/pull/4)); Driver Shared
comes from `10379c3a1a47f26c82dfbd66438a73baed5016d6`
([Driver Draft #8](https://github.com/shuyra-arche/driver/pull/8)). Those parent
packages select the same Execution provider and avoid duplicate `links`
ownership. They are standalone dependency selections, not new Vanllia component
gitlinks. The product keeps its existing System and Driver components.

Cargo deprecates `[replace]`; these bounded root selectors and their lock are
maintained by IOMMU. They do not propagate when Kernel consumes `iommu-hold` as
a library. When the CLI builds the IOMMU Server workspace, its native patches
select the product sources first: Cargo reports all seven exact replacements
as unused. The audit retained those warnings, as well as unused broad-overlay
patch warnings; the result ledger records the seven replacement messages.
There is no replace/patch conflict. Original and replacement provenance in a
Cargo lock is not an additional active compiled owner; the 24 target graphs
above are the owner checks. Dependency updates must repeat raw, active-owner,
CLI overlay and Kernel-consumer checks. Remove the corresponding selectors
only after the public manifest pins converge and those checks pass without them.

The previous `90f59ef` product result remains byte-for-byte in the
[ledger at Vanllia `72aa8f6`](https://github.com/shuyra-arche/vanllia/blob/72aa8f6a02ce333b8505d091c1455d87c46b5fe9/docs/sdk-consumer-results.json)
(SHA-256 `a73093dc824f1072d65f0e46387b9d95152a6b3be95bec929aee3ab85c6f0588`).
The [validation text at that commit](https://github.com/shuyra-arche/vanllia/blob/72aa8f6a02ce333b8505d091c1455d87c46b5fe9/docs/sdk-consumer-validation.md)
retains its then-current standalone failure. The current ledger contains only
the new `ba78aa6` artifacts and run.

## Pipe native exit and reclamation

The installed OpenSBI input was independently checked with `sha256sum` and
`wc -c`. During this local test only, the root used:

```toml
[boot.firmware]
path = "/absolute/path/to/installed/opensbi/generic/fw_dynamic.bin"
sha256 = "8f34d231fde9554e1a3c511bb1efac489da310ab0462504c512994601e12d213"
bytes = 273472
```

The actual local path was supplied by the installed QEMU/OpenSBI tool bundle;
the fragment was removed after testing and the public manifest was compared
byte-for-byte with its saved pre-test version. No machine path or firmware
binary is published in the product configuration. The hash identifies the
installed test input; it is not a new firmware redistribution claim.

QEMU `10.0.13 (Debian 1:10.0.13+ds-0+deb13u1)` ran `virt`, 256 MiB and one CPU,
with a 60-second timeout. Evidence is retained under
`.vanllia/runs/1793915796978586597-98243-0/`: `input.json`, `serial.log`,
`verdict.json`, and `process/{execution,exit,result}.json`. The process finished
in 1,408 ms, code 0, no signal, with `result = Ok(())`.

The unchanged typed native verifier accepted test Task 5's Reap/0 exit,
retirement acknowledgements for test Task 5 and Pipe Task 4, and System's
completion with reason 0. The VM exited naturally with code 0. Diagnostic prose
was not used to infer a pass. The verdict binds the exact Kernel, initrd and
firmware hashes and records `passed = true`, `error = null`.

This is a fresh single-core Pipe check. Previous multi-core/full-product runtime
and hardware discovery results retain their own older pins; this change does
not supply a new full-product QEMU or block I/O verdict.

## Retained startup constraints

The legacy Execution adapter still borrows initial arguments from the initial
Task's stack, and its existing entry assembly subtracts eight bytes from the
stack. Those argument-lifetime and RISC-V stack-alignment issues remain open for
the separate Runtime design. Legacy termination can leave a Client TLS `Live`
record until address-space destruction. The Pipe verifier checks the native
Exited → Retired transitions and System completion; that acknowledgement does
not establish per-Task TLS retirement or complete memory reclamation. No second
heap, TLS registry or task cleanup owner was introduced to hide these constraints.
