# Ownership follow-up validation

Base: Vanllia #6 `ff2c059c7df1545afedb95eb74dce71f40ca3f8d`.
Local branch: `codex/acceptance-ownership`.
Final implementation review tree:
`1fa922c9e655848d2b071e88e5a6ff2f3b809cf5`.
This record is added after review; it changes no implementation or test.

## Host evidence

Linux x86_64, Python 3.12.14 and Nushell 0.107.0:

```sh
python3 -B -m unittest discover -s tests -t .
```

Final result: **320 passed**, no skipped cases. The selected checkout has no
initialized `legacy/sqware` or provider checkout. Test infrastructure copies
the scripts and runtime package into temporary skeletons and uses executable
Cargo/Git/QEMU stubs; feed/NBD tests use local sockets. No real QEMU or Cargo
build ran.

- 305 original class/method identities are discovered exactly once. Only the
  four split Entry classes are mapped back to the original Entry owner.
- All 449 original `self.assert*` call ASTs match the original #6 bodies. Their
  per-case SHA-256 fingerprints are now frozen in `tests/assertion-baseline.json`
  and checked by the discovery test; deleting an assertion changes the hash.
- A one-time comparison against the actual #6 Git source found 60 unchanged
  oracle/observer function/class ASTs. This is source-migration evidence, not a
  permanent freeze on those implementations.
- The entire moved Nu verdict body equals #6 after normalizing only the three
  Python dispatch sites and module names. The controller's error-to-failure
  adapter ensures a verifier exception still reaches helper cleanup.
- Four synthetic confirmation-oracle CLI invocations pass with the third
  argument equal to `invalidate`, `reset`, `release` or `reference`.
- The new PID test observes the actual Python feed helper during the synthetic
  runner call and its exit after cleanup. A post-suite process scan found no
  live acceptance feed helper.
- Structural negatives cover ordinary/relative/from-package reverse edges,
  test-data implementation imports, stdlib trace collision, path manipulation,
  registry injection and dynamic oracle import access.
- A two-connection synthetic external script confirms that its module state is
  fresh per connection; built-ins retain normal package identity.
- Staged whitespace checks pass. Production Cargo files, locks, composition,
  bootstrap and provider gitlinks have no diff against the base.

The count comparison and remaining runtime limitations are detailed in
[acceptance-ownership.md](acceptance-ownership.md). The final total is
305 retained plus 15 new host cases; it does not include Rust product-contract
tests from the parallel line.

## Fresh independent model group

All three are genuine OpenCode Go `deepseek-v4.1-flash` sessions dedicated to
this cohesive follow-up:

- namer: `vanllia-boot-organization-namer-097c647c-3817-4f07-a087-5cb5aea9d9c9`;
- worker: `vanllia-boot-organization-worker-a7e7c689-996a-4a12-b516-bbc1708d014c`;
- reviewer: `vanllia-boot-organization-reviewer-a7df1dec-ac5d-4fdf-8d94-62c1e8a0803f`.

The interrupted namer request was found complete on reconnect and was not
repeated. The worker supplied the runtime migration implementation; the
coordinator applied source patches, adapted the host tests/Nu ownership,
corrected implementation issues and ran all checks. Initial review prompted the
continuous assertion-fingerprint check and clearer driver-precondition boundary.
Two mistaken review findings were explicitly withdrawn after source/CLI evidence.
Final review response `chatcmpl-361cda04a33746c28f44fba9e7c1beab` was **READY for
the cohesive local commit prepared for parent integration, not publication or
merge**. The reviewer inspected supplied snapshots and evidence; it did not run
tools or independently claim guest results.

## Integration status

No new PR or remote branch is published for this follow-up, and no published
history is rewritten. Product-contract #8 remains a separate boundary at
`5ebafc855980cf6ff7115d4a9ae6980d9b76dd38`. Its changed-path intersection with this
follow-up is README only; its Rust/Cargo/test changes are untouched. Preserve
both command sections when the parent assembles the chosen integration branch.

No merge, close, branch deletion, force push, QEMU/SMP, real product/Kernel/
firmware build or guest acceptance is performed. Product and scene migrations
still gate complete sqware independence and archive.
