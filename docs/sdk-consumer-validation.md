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
| IOMMU | `90f59ef91d3c4c157fb78bb691991ee7abe4f175` |

The SDK Git sources are pinned in `vanllia.toml`; the CLI's real
`fetch --update` generated `vanllia.lock`. Other component gitlinks and the
Unit source pin remain unchanged. IOMMU uses the canonical repository URL
`https://github.com/shuyra-arche/driver-iommu.git`. Its explicit source replaces
the nested Driver IOMMU provider during CLI discovery. The IOMMU candidate's
Server and Unit files match its default-branch base; the relevant dependency
change makes `iommu-hold` use the pure Kernel API.

All six development source overrides were reset before fetching. The resulting
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
| Default Login + Daemon | 17 | 14 | `1793913263541098745-62856-0` | `62af1ad1b8659c0acaa0e3a8fb8370c786ae5e5023ff6636620c2c47fcb122a4` |
| `virtio-blk` closure | 5 | 6 | `1793913412685619874-68771-0` | `86ee368aa9cc160a8d0e4b0ab6af24bd3545f5edcd30cc8d08e4f7c8e6a6254c` |
| `test.pipe-roundtrip` | 3 | 4 | `1793913480955620851-72181-0` | `1f423543d225fe44d65f6225b07d02e39194dcc157999f3895d6bbfc6b41874f` |

Kernel is a separate boot artifact in each receipt, not one of the runtime image
counts. Its identical ELF SHA-256 is
`26bf3ed691e229a0674431727736d0e3b6cbd3a1171d1fd9e659bb2e27dc6be6`
(1,797,544 bytes). The optional driver closure actually compiled System, Hub,
IOMMU, PCI and VirtIO block. Default and driver build receipts have no firmware;
only the Pipe test artifact includes the explicitly selected local firmware.

Each artifact directory retains `receipt.json`, `vanllia.lock` and its copied
`cargo-N.lock` files. All recorded image, Kernel, initrd and Cargo lock lengths
and SHA-256 values were independently checked against their files. The compact
[result ledger](sdk-consumer-results.json) records receipt hashes, copied locks
and graph owners for all three selections. These are new published build
receipts. The earlier development build rejected by the source-change guard
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
`kernel-client` owner at `components/kernel/client`; Kernel has none. Kernel's
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
protocol package came from the new pin. The raw standalone IOMMU Server build
with its older unoverlaid SDK dependencies has six type-identity errors; this
validation proves the supported CLI composition, not a standalone Server pass.

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
`.vanllia/runs/1793913494654980316-72181-0/`: `input.json`, `serial.log`,
`verdict.json`, and `process/{execution,exit,result}.json`. The process finished
in 1,358 ms, code 0, no signal, with `result = Ok(())`.

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
