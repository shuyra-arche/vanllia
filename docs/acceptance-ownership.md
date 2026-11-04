# Acceptance ownership and coverage

Base: Vanllia #6, `ff2c059c7df1545afedb95eb74dce71f40ca3f8d`.
This is one cohesive follow-up prepared on `codex/acceptance-ownership`. It does
not alter a published branch, create intermediate PRs, or merge another line.

## Ownership

`scripts/qtest.nu` owns build/runner invocation, board selection, helper lifetime
and evidence archive cleanup. `acceptance/scenes.nu` supplies its scenario facts.
`acceptance/verdicts.nu` owns the final whole-run verdict and invokes namespaced
Python oracles from an explicit repository cwd. Its body is the #6 verdict body
with only module dispatch changed; it does not start a helper or VM.

Python drivers own orchestration, capture/summary publication and runtime input
paths. Memory fresh/TLB/race adapters call the same streaming judges exercised
by host tests. Stateful memory phase orchestration stays in its driver Harness;
final reclamation, wait/wake/exit counts and slot-identity predicates live in
`oracles.memory_transfer.phases`.
Transport owns bytes, lines and EOF, without a memory-scene prefix. Evidence
owns Kernel event observation and device-tree parsing. Shared hardware injection
constants belong to `contracts`, eliminating oracle-to-driver imports. The NBD
server is a runtime fixture; synthetic trace data lives exclusively under
`tests/fixtures`.

All internal imports use explicit packages. There is no generic `trace.py`,
`sys.path` mutation or internal fake package registration. Only an explicitly
requested external `--feed-script` uses an extension loader supporting that
user script's relative imports. Built-in drivers use `--feed-driver` and normal
module identity. No new installation step or `PYTHONPATH` is required.

Runtime observers take absolute `VANLLIA_DIAGNOSE`; standalone idle takes
`--root`. Source-file depth and the capture directory no longer discover the
repository or input trace. Archives still live beside the explicit capture;
the Kernel's `sqware-diagnose.jsonl` export filename remains a protocol fact.

`Harness.arm`, `fault_killed` and `never_exits` intentionally remain driver-side
preconditions: their observations determine when the next input gate may open.
The phase record still contains the original `faults` count sampled at arming.
These observations are not advertised as complete final fault verdicts. Fresh,
TLB and race have separate scene-specific final fault checks in their streaming
oracles. Final phase count/exit/reclamation/slot checks use the extracted oracle.

The old feed launch recorded a background shell PID because `&&` grouped with
`&`. It could leave the actual listener alive after host tests. The coordinator
now records the Python process PID. A new test observes that PID while the stub
runner executes and verifies it exits during cleanup. Failed helper startup
also terminates the recorded process. No guest behavior or proof is substituted.

## Coverage comparison

Run from the repository root:

```sh
python3 -B -m unittest discover -s tests -t . -v
```

| #6 coverage | Original cases | Current owner |
| --- | ---: | --- |
| Entry/build, board, normal runner, qtest | 22 | `tests/boot` (four concerns, shared support has no cases) |
| Device recovery predicates | 118 | `tests/oracles/device_recovery` |
| NBD fixture and QEMU executable-wrapper stubs | 8 | `tests/transport` |
| Daemon recovery predicates | 80 | `tests/oracles/daemon` |
| Fresh/TLB/race evidence predicates | 64 | `tests/oracles/memory_transfer` |
| Kernel observer and final phase settlement | 13 | `tests/evidence` |
| **Retained original total** | **305** | **Discovered exactly once** |
| New import/discovery structure checks | 6 | `tests/structure` |
| New process PID and early-input checks | 2 | `tests/boot` |
| New feed/package/extension/failure checks | 6 | `tests/transport` |
| New explicit diagnostic-input checks | 1 | `tests/evidence` |
| **Current total** | **320** | |

`tests/coverage-baseline.json` records every original class/method and original
path. The structure test canonicalizes only the four split Entry owner classes
and requires each original case exactly once. It does not accept a matching
total with missing or duplicated cases. `tests/assertion-baseline.json` freezes
the original per-case fingerprints from #6; the same structure test now checks
all 449 original `self.assert*` call ASTs continuously, with a negative control
showing that deleting an assertion changes the fingerprint. Expected values are
not computed from the new implementation. Sixty original oracle/observer
definitions were separately compared once against #6 and had identical ASTs;
that one-time source audit is not a continuously enforced implementation freeze.
Extracted phase predicates retain the original checks and messages with
explicit observation/state arguments. The full Nu verdict body matches #6
after normalizing only Python module dispatch.

Explicit external extensions retain fresh module state for each accepted
connection, as the legacy helper did; built-ins retain ordinary package identity.
A local-socket regression checks two connections to a stateful synthetic user
script. Four separate synthetic CLI checks also confirm the retained
confirmation oracle accepts its third positional input as the step name.

The import-direction check inspects actual imports, including relative and
`from package import child` forms. Synthetic forbidden edges demonstrate that
it rejects oracle-to-driver, runtime-to-test, test-fixture-to-implementation,
unqualified trace, `sys.path`, and internal registry-injection dependencies.
Importing the runtime package leaves the already imported stdlib `trace` intact.

`acceptance/source-files.json` retains the original immutable sqware commit/blob
identities and maps the #6 file locations to their new owners. It is provenance,
not a claim that reorganized source files still have the old raw blob bytes.

## Parallel product contracts and runtime boundary

Product-contract #8 remains at
`5ebafc855980cf6ff7115d4a9ae6980d9b76dd38`, based on #5
`2643ee792b9fc791e24bbfc3a8efedc1f83fd69a`. It owns Rust composition declaration,
inventory and startup contracts, its Cargo dev edge and
`docs/product-contract-tests.md`. This follow-up changes none of those files,
Cargo locks, providers, product coordinates or the retained equivalence suite.
The 305/319 totals are Python host coverage, not a claim about #8's Rust gates.
When the product lines integrate, retain #8's five contracts and five existing
equivalence cases, plus #7's sixth Shell/Cat comparison. README integration must
retain both the product commands and this unified Python command.

Only `product` remains enabled by the runtime entry. Other scene deployments
and guest fixtures are still pending. No QEMU, SMP, real product/Kernel/firmware
build, idle or guest acceptance is performed or claimed here. Firmware source
and licensing constraints remain those in [firmware.md](firmware.md); no binary
is copied. Host restructuring does not remove the remaining sqware product,
aggregate or fixture dependencies and does not permit repository archive.
