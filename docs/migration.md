# Build tools consolidation

The host tools are imported from image-builder commit
`82f81b121a9c8f0590ceceb51c77fa0369355217` into Vanllia. The former
`components/image-builder` gitlink is removed. Other provider pins and
`vanllia.lock` are unchanged. The root Cargo lock is copied without dependency
updates from that image-builder revision.

| Former image-builder path | Vanllia path |
| --- | --- |
| `cli/src/` | `src/` |
| `cli/tests/` | `tests/cli/` (explicit Cargo test targets) |
| `src/`, `tests/`, `examples/` | `crates/build/` |
| `cli/README.md` | `docs/cli.md` |
| `docs/workflow-contract.md` | `docs/workflow-contract.md` |

Install with `cargo install --path . --locked --target x86_64-unknown-linux-gnu`.
Library users retain package `vanllia-build` and crate `vanllia_build`, now at
`crates/build`. The lifecycle tests use the new local paths. The CLI's discovery,
public APIs, publication receipts and runtime policy are unchanged. The fixture
workspace remains independent; its relative paths and the examples' `library`
and `{tool}` paths still resolve within `crates/build`.

Host commands now default to the host target. RV64 flags remain under the
target-specific Cargo table, and guest builds continue to pass an explicit
target. Kernel, System and service repositories remain independent workspaces.

The optional `firmware/` build recipe is removed. Identified external firmware
inputs, hash/byte verification and QEMU support remain; see [firmware inputs](firmware.md).

## Legacy test audit still open

This migration preserves all historical Python tests, assertion fingerprints,
coverage baselines and `acceptance/source-files.json`. None are retired merely
to reduce the language percentage. The following ownership audit remains needed
before broader deletion:

- `acceptance/{oracles,evidence,transport,drivers}` and their self-contained tests:
  preserve their evidence and process-lifecycle contracts.
- `tests/boot` and release checks: identify assertions tied only to removed Nu
  entry points separately from cancellation, reaping, timeouts and evidence checks.
- `composition/tests`: assess coordinates, namespace, deployment, inventory and
  Unit contracts individually against the current implementation.
- `docs/generate-composition-inputs.py`, `acceptance/scenes.nu`,
  `acceptance/verdicts.nu`, `scripts/check-evidence.py` and
  `scripts/record-evidence.py`: retain pending caller and semantic review.

The full historical Python suite is not a passing product gate. Record its
failures separately from the current Rust CLI/library and product lifecycle tests.
