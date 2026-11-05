# Observed build and host-test records

`scripts/record-evidence.py` records one invocation of an existing entry. It
does not accept a supplied result, `--passed`, arbitrary command or artifact
path. Input inventories remain independent and unchanged. Join a run to its
input by `input.inventory_sha256` and `input.vanllia_commit`.

## Use

Run from a clean committed checkout with the exact initialized source
checkouts required by [the input inventory](version-release-plan.md). Python
3.11+ and Git supporting `--no-lazy-fetch` are required. Build/Rust runs also
need the repository's installed Rust toolchain; build and Python host tests
need Nushell. The recorder does not install tools or initialize submodules.

```sh
candidate_sha=$(git rev-parse HEAD)
record_dir=$(mktemp -d)
python3 docs/generate-composition-inputs.py --commit "$candidate_sha" \
  --output "$record_dir/inputs.json"

python3 scripts/record-evidence.py --inputs "$record_dir/inputs.json" \
  --output "$record_dir/build" --action build --profile release
python3 scripts/record-evidence.py --inputs "$record_dir/inputs.json" \
  --output "$record_dir/python" --action python-tests
python3 scripts/record-evidence.py --inputs "$record_dir/inputs.json" \
  --output "$record_dir/rust" --action rust-tests
```

Each invocation independently creates a new evidence directory outside the
source checkout. Existing directories are refused. A failed command returns
nonzero and attempts to retain its record/logs; subsequent invocations are separate and
must be checked separately. `--root` selects another initialized checkout.
`--profile debug|release` applies only to `build` and defaults to `release`.

| Action | Existing entry actually executed |
|---|---|
| `build` | `nu scripts/build.nu <profile>`; its `--plan` is also captured before building |
| `python-tests` | Recorder's Python executable with `-B -m unittest discover -s tests -t .` |
| `rust-tests` | `cargo test --locked --offline -p vanllia-composition -p vanllia-image --tests --target x86_64-unknown-linux-gnu` |

For build/Rust actions, the recorder explicitly selects `RUSTUP_TOOLCHAIN`
from the committed root `rust-toolchain.toml`, including for nested Cargo
invocations. This replaces an inherited selection and is recorded in
`configuration.rustup_toolchain_override_from_snapshot`. The actual rustup,
rustc and Cargo probe outputs are retained; an unexpected active selection
prevents execution. Other selected environment overrides are recorded, never
the entire environment or credentials. The host-test target matches the
existing Linux x86_64 entry; other host targets are not implemented.
The Python action keeps the inherited Rust selection unchanged; it records
the selected environment values and does not invoke the Rust test entry.

## What is checked and recorded

The recorder first requires root HEAD to equal the input commit and root to
be clean, including ordinary untracked files. Its own bytes must equal the
recorder committed at that input. It reuses the existing inventory generator
and compares its regenerated JSON value to the supplied JSON. A manually
edited source/version/approval assertion cannot become a verified input.
The hash binds the supplied file bytes, including formatting.

It checks HEAD and clean status for every expanded inventory checkout before
and after the run. Unlike a pure object inventory, execution requires those
checkouts at the selected commits. The legacy child gitlinks remain outside
the inventory expansion; they do not acquire execution checks here. Git calls
disable replacement objects, lazy fetching and optional locks. There is no
second Cargo dependency resolver or new graph check.

`record.json` has `schema_version: 1`, `kind: execution-evidence`, and:

| Field | Observed source |
|---|---|
| `input` | Commit/tree, supplied inventory SHA-256 and successful replay/binding status |
| `producer` | SHA-256 of the recorder actually invoked, checked against committed bytes |
| `source` | Before/after checked checkout commits; `verified-before-and-after` only when both checks succeed |
| `configuration` | Input inventory hash (binding committed configs/locks), configured root toolchain records, selected environment values, profile and captured build-entry plan |
| `tools` | Version-probe argv, resolved executable path, return code and stdout/stderr log fingerprints |
| `execution` | `exited` means a process returned, including failures; actual argv/cwd, UTC start/end, elapsed seconds, process return code and log fingerprints; otherwise `not-run` |
| `artifacts` | For successful builds only: initrd/Kernel paths discovered from build-entry JSON, resolved paths, regular-file checks, byte sizes and SHA-256; missing entries remain explicit |
| `missing` | Failed prerequisites, malformed producer output, unavailable artifacts or failed post-run source checks |
| `not_evaluated` | Explicit exclusions, including dependency graphs, test counts/coverage, freshness, reproducibility, firmware, runtime and approval |

Each log fingerprint names a relative file inside that evidence directory and
includes its byte size and SHA-256. The supplied input bytes are copied as
`inputs.json` alongside the logs; no original inventory is edited.
Tool versions are observations from the configured host, not compiler binary
attestations. Global/user Cargo configuration, wrappers and the complete host
environment are not exhaustively captured. Ignored files are outside clean-Git
checks; before/after checks do not establish isolation throughout execution.

Example state fields from a completed observation:

```json
{
  "schema_version": 1,
  "kind": "execution-evidence",
  "status": "observed",
  "is_release_manifest": false,
  "release_approved": false
}
```

`observed` means the command returned zero and required recording/binding
checks completed. `failed` preserves a nonzero command return code.
`incomplete` means a prerequisite, output or source binding is missing, even
when a process returned zero. Recorder CLI returns zero only for `observed`;
otherwise it returns nonzero. Test case counts and assertion coverage are not
derived from console text. Process success is not approval of a release gate.
The overall status, `source.status` and `missing` must be read together; a
nonzero process can also have an incomplete source binding. `execution.status`
only describes process lifetime and does not assert output validation.

Artifact paths come from the actual build-entry result rather than a fixed
legacy path. The record observes those files after execution; it does not
establish a fresh compile, cold-cache reproducibility or that a path could not
have contained the same bytes before the run. It does not parse ELF or initrd
contents; retain the separate real consumer/parser audit for those claims.

The bundle is local evidence, not a signature or tamper-proof attestation.
Keep its directory under the operator's control; racing writers are not isolated.
Verify retained file hashes before relying on it. I/O failure or abrupt termination can leave
an unfinished directory without a final `record.json`; such a directory is
incomplete evidence. No firmware is selected or copied, and no QEMU/SMP entry
is provided. Source/test code and installed host tools remain trusted inputs.

## Validation and remaining work

```sh
python3 -B -m unittest tests.release.test_evidence -v
```

The recorder tests use small committed Git fixtures. Their build/Rust tools
are explicit synthetic doubles; their Python success/failure tests execute
real tiny unittest suites. They verify bindings, observed hashes, fixed argv,
failure/missing states and rejection of edited approval claims. They are not
product/Kernel build evidence. The full host discovery also preserves the
original acceptance assertion fingerprints.

This implements independent build/test observations. Joining multiple runs,
evaluating release gates or coverage, authenticating tools, validating selected
dependency graphs, selecting firmware, runtime acceptance, assigning versions,
publishing and signing release records remain separate work. Successful local
records do not authorize the previously rejected Vanllia merge.
