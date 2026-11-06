# Vanllia

`vanllia` discovers provider-owned `unit.toml` files, selects the required Units,
builds Cargo bins and packages their binary declarations and images into initrd.
Deployment exists only in host memory. System reads the initrd and creates
startup instances; requested Units remain available for runtime creation.

```sh
cargo install --path . --locked --target x86_64-unknown-linux-gnu
vanllia fetch --root /path/to/product
vanllia check --root /path/to/product
vanllia plan --root /path/to/product
vanllia build --root /path/to/product
```

Use a nightly Cargo with `resolver.lockfile-path`, the RV64 target and LLVM
tools (`llvm-objcopy` on PATH). `rust-toolchain.toml` pins nightly-2026-09-04; a clean
run under that pin is still needed before claiming it as a verified environment. Cargo provides compilation, dependency resolution
and caching. The former `vanllia-build` executable has been retired.

## Product configuration

The CLI searches upward for an independent `vanllia.toml`:

```toml
schema = 1
units = ["pipe"]
[boot]
system = "system"
[boot.kernel]
image = "shuyra"
[platform]
target = "riscv64gc-unknown-none-elf"
profile = "release"
[sources.runtime]
git = "https://github.com/shuyra-arche/system.git"
rev = "REPLACE_WITH_A_VERIFIED_FULL_COMMIT_SHA"
[sources.pipe]
path = "components/pipe"
[runner]
machine = "virt"
memory = "256M"
cpus = 1
timeout = 60
```

Repeated `--unit ID` replaces the default selection. System is always included.
`check` and `plan` are read only, including locks and source preparation.
`build` prepares missing sources; `--offline` requires prepared sources and
cached inputs. Only the entry configuration supplies product defaults.

The root, explicit source repositories and registered submodules form the source
scope. TOML files and Cargo bins are discovered automatically. Outputs, caches,
symlinks and unregistered nested repositories are excluded. Explicit sources
replace registered submodules with the same origin; conflicting explicit copies
fail. Unit IDs and Cargo bin names are independently unique.

Git sources accept `branch`, `tag`, `rev`, or the remote default, then lock an
actual commit in `vanllia.lock`. `fetch --update` refreshes the lock explicitly.
Failed preparation preserves the previous lock. Managed checkouts must be clean.
Cargo owns Rust dependency locks; Vanllia writes resolution locks outside the
developer source trees.

## Local development

```sh
vanllia workspace --root /path/to/product --source runtime=/work/system
vanllia workspace --root /path/to/product --manifest /work/system/server/Cargo.toml
vanllia workspace --root /path/to/product --reset-source runtime
```

Choices live in ignored `.vanllia/overrides.toml`; shared source policy and its
Git pin remain intact. Cargo-native patches bind selected local packages during
the build. Optional editor output contains the same Cargo configuration, linked
workspace and target for rust-analyzer. Copy its settings into the editor; the
CLI leaves existing editor settings untouched. Uncommitted local edits are used
directly. Build receipts record the actual source hashes.

## Unit declaration

```toml
schema = 1
id = "test.pipe-roundtrip"
[loader]
image = "test-pipe-roundtrip"
space = "user"
[control]
creation = "startup"
ending = "transient"
requires = ["pipe"]
after = ["pipe"]
```

The shared Unit SDK validates loader, namespace, identity, control and supply
policy and encodes a versioned record. Quote decimal or hexadecimal values above
TOML's signed integer range. Cargo metadata supplies package/bin ownership;
several Unit IDs can share one image. `requires` grows a finite inclusion set and
can be cyclic. `after` waits only for selected Units and never expands selection;
unknown references and selected wait cycles fail. `creation = "requested"`
retains a Unit without initially creating it. Every selected Unit is packaged.

Runtime images are stripped once; debug executables remain on the host. Raw
resources can be declared in `[package.metadata.vanllia.resources]` by name/path.
System declares its generated boot-ledger supplies in the same table's
`runtime_resources` array; these remain runtime resources instead of file inputs.
Construction targets must be in the required inclusion set. Kernel and firmware
are separate boot artifacts. Build publication atomically exposes one complete
directory containing initrd, hashes, native Cargo locks and source evidence.
A failed build leaves previous artifacts intact.

## Tests and exact artifacts

New integration cases are Rust guest bins with `unit.toml` and `test.toml`:

```toml
schema = 1
name = "pipe.roundtrip"
unit = "test.pipe-roundtrip"
expected = "clean"
timeout = 60
```

```sh
vanllia test -- --list
vanllia test -- pipe.roundtrip
vanllia run --artifact /path/to/.vanllia/artifacts/BUILD
```

Each case builds its test Unit, necessary providers and System. The runner
requires `[boot.firmware]` with either `path` or HTTPS `url`, a SHA-256 and byte
length. It validates artifacts and snapshots the exact bytes into a fresh run
directory. Cancellation and timeout stop and reap the owned process group.
`run --artifact` uses the saved artifact without rebuilding or adding a test Unit.

System forwards versioned typed frames containing native task exits and
acknowledged retirement. Verdicts require the expected exit cause/reason, every
instance reclaimed and successful System completion. Supported expectations are
`clean`, `{ exit = 17 }` and `fault`. Fault matches the native fault category;
it does not identify an ISA exception code. Diagnostic prose cannot produce a
pass. Missing, truncated or invalid protocol evidence fails. Kernel event logging
is not required.
The serial adapter accepts UART CRLF endings while preserving strict payload
decoding. A run's `verdict.json` binds the case and native result to exact artifact
hashes alongside the original serial evidence.

Host tests include real RV64 fixture builds, tampered artifact rejection, source
locks, local overlays, editor resolution and receipt state transitions. Product
runtime checks belong to the consuming repository and must be run against its
selected source pins. Retained old scenario code is not a current test catalog.

```sh
cargo test -p vanllia --target x86_64-unknown-linux-gnu --locked
cargo clippy -p vanllia --all-targets --target x86_64-unknown-linux-gnu --locked -- -D warnings
```
