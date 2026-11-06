# Retained acceptance machinery

This Python package holds scenario drivers and evidence judges. The current
product entry is the independent `vanllia` CLI described in the root README.
The former Nu boot entry has been removed; do not use historical `cargo image`
or `scripts/qtest.nu` commands.

| Directory | Responsibility |
| --- | --- |
| `drivers/` | Scenario orchestration and runtime input |
| `oracles/` | Evidence predicates, without helper or VM ownership |
| `evidence/` | Kernel event observation and device-tree parsing |
| `transport/` | Framing, capture and explicit feed extensions |
| `fixtures/` | Runtime NBD fixture |
| `contracts/` | Shared injection and register values |
| `../tests/` | Synthetic inputs, executable stubs and host tests |

Internal imports are normal packages; no installation or `PYTHONPATH` is needed.
Oracles must not import drivers. Synthetic trace builders stay under tests.
Runtime diagnostic paths are explicit, rather than inferred from source depth.
`source-files.json` retains original source/blob attribution; keep it intact.

Run the self-contained groups from the repository root:

```sh
python3 -B -m unittest discover -s tests/oracles -t .
python3 -B -m unittest tests.transport.test_feed
python3 -B -m unittest discover -s tests/evidence -t .
```

These checks do not boot QEMU or validate current product integration. Full
`python3 -B -m unittest discover -s tests -t .` still includes legacy entry and
release-script checks that refer to removed build scripts. Keep those failures
visible until that test code is maintained separately; do not count full discovery
as a passing gate. Current runtime gates are documented in
[product lifecycle tests](../tests/product-lifecycle/README.md).
