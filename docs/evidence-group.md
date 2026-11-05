# Read-only consistency of an evidence group

`scripts/check-evidence.py` checks one expected input snapshot and exactly one
retained `build`, `python-tests` and `rust-tests` record. It reads files and
prints JSON; it does not execute recorded argv, invoke Git/Cargo/Nu, generate
new run records, copy artifacts or modify the supplied evidence. It uses only
Python's standard library, without a second dependency resolver or builder.

```sh
python3 -B scripts/check-evidence.py --inputs "$record_dir/inputs.json" \
  --record "$record_dir/build" \
  --record "$record_dir/python" \
  --record "$record_dir/rust" \
  --profile release
```

`--inputs` is the reference snapshot chosen by the reviewer/operator. Repeat
`--record` for retained directories containing `record.json`. Record order is
irrelevant; missing, duplicate and unexpected actions are errors. The expected
build profile defaults to `release`; `--profile debug` explicitly checks a debug
group. The two test actions retain their own fixed host-test commands and null
profiles. There is no option to skip a required action or declare it passed.

## Checks

For each supplied record, the checker requires the supported version-1 input
and execution-record formats, successful recording state, no missing items,
zero process return codes and these associations:

- Input commit/tree and exact snapshot SHA-256 match the expected snapshot.
  The retained `inputs.json` copy hashes to that same value. The recorded
  inventory-replay stdout decodes to the same snapshot value, allowing the
  original input formatting to differ from generator formatting.
- Stored source checks before and after name exactly the expanded snapshot
  repositories/commits with clean status. This is comparison of retained
  observations; it does not perform a fresh source checkout check.
- Configuration points to the same snapshot hash and configured root toolchain.
  Build/Rust selections match its channel, including the stored active-toolchain
  probe output. The expected build profile matches the captured plan and result;
  `configuration.build_plan` matches the retained plan stdout. This is not proof
  that the plan's product/Kernel command lists were executed or caused the
  observed artifact bytes.
- All retained process observations have the expected fixed action/probe argv,
  consistent recorded cwd and both stdout/stderr files with matching sizes and
  SHA-256. The build plan is taken from its retained producer log; Kernel build
  arguments are not reconstructed by this checker.
- Successful build stdout reports the same initrd/Kernel paths as the two
  artifact entries. Their reported/resolved paths must still agree, the files
  must remain regular files, and their sizes/hashes must match. No artifact
  path can be substituted through a checker CLI option.

Across the three individually valid records, recorder fingerprints and output
fingerprints of each shared version probe must agree. The recorded environment
dictionaries must agree except for `RUSTUP_TOOLCHAIN`: the Python action keeps
its inherited selection, while build/Rust actions must use the snapshot channel.
This is a conservative comparison of already-recorded values, including paths
and job settings; it does not infer equivalence between different host setups.
Unrecorded global/user configuration remains outside this check.

Log/input references must stay inside their own evidence directory, including
when resolved through symlinks. Artifact paths use the original recorded
absolute paths. Relocation, durable artifact copies and publication packaging
are not implemented. A removed or replaced `target/` artifact makes the group
inconsistent until the correct retained bytes/path association is available.

## Result and missing evidence

Exit 0 means the retained evidence group is internally consistent. Exit 1
means missing, malformed or inconsistent evidence. CLI argument errors use
argparse's nonzero exit. The JSON report includes:

```json
{
  "schema_version": 1,
  "kind": "evidence-group-consistency",
  "status": "inconsistent",
  "is_release_manifest": false,
  "release_approved": false,
  "required_actions": ["build", "python-tests", "rust-tests"],
  "expected_build_profile": "release",
  "errors": [
    {"record": "rust-tests", "message": "missing required action"}
  ]
}
```

With a valid expected snapshot, the report names its commit/tree/snapshot hash.
Each successfully parsed `record.json` adds its raw SHA-256; unreadable or
unparseable records have no file-digest claim. `records[].status` describes that record's
individual checks; cross-record differences appear in the top-level `errors`.
Only top-level `status: consistent` means all checks completed without errors.
The checker continues across records and reports the first failed check within
each invalid record, plus missing/duplicate-action and cross-record errors.
An absent `record.json`, absent log/artifact or malformed JSON is an error,
not an omitted success. Duplicate JSON keys are rejected to avoid ambiguous
commit/action/configuration interpretation. No report file is created unless
the caller explicitly redirects stdout outside the tool.

## Scope and validation

Checking an old group does not require current HEAD to equal its historical
commit. The report always names the supplied group's subject commit. A check
performed by newer code does not relabel old executions as tests/builds of
that newer commit. Rechecking a group does not rerun any command.

```sh
python3 -B -m unittest tests.release.test_evidence_group -v
```

The unit fixtures deliberately contain synthetic retained records/files and
test consistency behavior, not authenticity of execution. They cover complete
groups, changed commit/tree/input, missing/duplicate actions, source/config/tool
differences, log/artifact corruption, path association and malformed data.
The checker also accepts the actual retained three-action group generated at
`08cf4046bc6d91a490a7cdf98cc4910d8dc315f6` when its original files are present.

`not_evaluated` explicitly retains execution authenticity/signatures, test
counts/coverage, runtime, dependency graphs, artifact freshness/retention,
complete environment/reproducibility, firmware and release approval. A
coherently fabricated unsigned record is not authenticated by this tool.
Expected snapshots and retained evidence still need trusted provenance. Hash
checks are local reads, not isolation from concurrent or adversarial writers.
No runtime/SMP or coverage gate is evaluated, no version is selected and no
publication or previously rejected merge is authorized.
