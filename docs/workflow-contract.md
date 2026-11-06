# Retained workflow library contract

`vanllia_build::workflow::execute(config_path, state, action)` is the JSON
workflow library interface in `src/workflow.rs`. It is separate from the TOML
product interface of the current `vanllia` CLI. `src/workflow/config.rs` defines
the accepted schema and rejects unknown fields; `examples/` contains fixtures.
Callers select the configuration's fixed `RUSTUP_TOOLCHAIN` before execution.

## Inputs and local overrides

Sources use credential-free HTTPS URLs and full commit pins. Workspaces declare
manifests and the public packages whose resolved source must match. Cargo owns
resolution, dependency locks, compilation and caches. The workflow prepares
private snapshots and manifests; patches apply only there. Snapshot identity
includes configuration, source bytes, patches, locks and relevant build inputs.
Do not treat a context as reusable if those inputs change.

An override records a local source choice; checking it prepares and verifies the
actual Cargo graph. The user's checkout remains read-only: no reset, checkout,
lock rewrite or cleanup. Snapshots include tracked and nonignored untracked
files, deletions and executable modes. Symlinks and special files are rejected,
including source paths reached through symlink ancestors. Sources are checked
for changes while preparing. An overridden repository with gitlinks must instead
have independently declared sources. Clearing an override selects the baseline;
it does not restore bytes over the user's files.

State must be outside the library and every active or proposed override checkout,
including canonical aliases. The state path is checked before creating output.
`inspect` reads retained state without preparing sources or starting Cargo and
can inspect absent or reserved state. Mutating actions hold an operation lease.

## Runner and process ownership

Runner commands are argument arrays, not shell recipes. Supported substitutions
include `{sources}`, `{tool}`, `{initrd}`, `{kernel}`, `{firmware}`, `{evidence}`
and `{harts}`; target-test runners also receive `{executable}`. The declared runner
owns semantic acceptance; VM exit alone is insufficient. Host, target and scenario
tests remain distinct. Cluster execution runs its declared collaboration scenarios
and fails if none are declared. A provider's host tests do not silently become
product runtime tests.

Linux embedders must arrange child reaping with `workflow::process::initialize`
and wire cancellation to `workflow::cancel`. Execution owns child process groups;
timeout/cancellation stops and waits for them. A successful leader cannot hide
leaked descendants. If stopping cannot be confirmed, `STOP_UNCONFIRMED` reserves
the state against reuse, even if writing the failure diagnostic also fails.
Do not remove that reservation as a retry mechanism without establishing that
its resources have actually stopped.

## Publication and failure

Each attempt has separate outputs/evidence. Build and run first record
`outcome.json` with phase `prepared` and no success claim. Only the atomic
`current.json` pointer with phase `published` establishes deployment publication.
A failed attempt never substitutes an earlier initrd and preserves the previous
published pointer. Publication/cleanup failures remain failures; caches need not
roll back. `build` establishes artifact production; `run` additionally establishes
the declared runner's acceptance.

Output locks protect canonical aliases. Final bytes are synced in a sibling
temporary file and renamed. Persistent lock files retain inode identity; closing
the lease releases ownership. This is local filesystem atomicity, not a
cross-machine lock or power-loss durability guarantee. Returned paths identify
frozen artifacts. Static image coordinates are interned for the process lifetime;
long-running callers must bound distinct configurations.
