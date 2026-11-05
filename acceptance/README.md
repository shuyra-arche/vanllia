# Acceptance material

The runtime package is organized by responsibility; host tests have a separate
root package. Internal modules use ordinary package imports. No installation,
`PYTHONPATH`, sibling import search or synthetic module registration is needed.

| Owner | Responsibility |
| --- | --- |
| `drivers/` | Live scenario orchestration and output publication |
| `oracles/` | Evidence verdicts; memory judges may consume supplied live observers |
| `evidence/` | Kernel event observation and device-tree parsing |
| `transport/` | Serial framing, capture, repeating input and explicit extensions |
| `fixtures/` | Runtime NBD fixture service |
| `contracts/` | Shared injection/register constants |
| `scenes.nu` | Scenario facts shared by orchestration and verdicts |
| `verdicts.nu` | Whole-run evidence checks; never starts a helper or VM |
| `../tests/` | Host cases, synthetic evidence fixtures and executable stubs |

From the repository root, run all host tests with one standard discovery:

```sh
python3 -B -m unittest discover -s tests -t . -v
```

This command uses synthetic data, local sockets and executable stubs; it does
not boot QEMU. Structural checks enforce import direction and discovery of all
305 original cases exactly once. Synthetic trace builders live under
`tests/fixtures`, never in the runtime package. The old `trace.py` name is gone;
the Kernel observer is `evidence.kernel_events`, while device execution proof is
`oracles.device_recovery.execution`.

`source-files.json` maps original paths/blobs and the #6 locations to current
owner files. History remains anchored to
[`sqware 94e11461`](https://github.com/anreonyr/sqware/tree/94e114614bec53424ac55e640fc80cf19a3e7107/programs/tests)
and Vanllia #6; directory depth is not preserved as an interface.

## Runtime entry, pending authorization and execution

Once firmware has been selected as described in
[`docs/firmware.md`](../docs/firmware.md), the Vanllia-owned acceptance entry is:

```sh
nu scripts/qtest.nu --package kernel --scene product
```

It invokes **`cargo image build product release`**, resolves the last `ok …`
path printed by that build, and runs `cargo qtest` in `components/kernel`.
The serial feed, timeout/PID ownership, console capture, unique real entry-task
success proof and exact physical-frame baseline check remain the old contracts.
The Kernel test runner enables QEMU semihosting itself; `--board-only` continues
to omit `-kernel`, `-initrd` and semihosting options. Product acceptance runs the
release Kernel test target. An explicitly selected debug **image** profile does
not change that existing test-target policy and is not a validated whole-system
configuration.

Built-in live drivers use module selection, for example:

```sh
nu scripts/qtest.nu --scene product --feed-driver acceptance.drivers.shell.smoke
```

The feed process runs `python3 -m acceptance.transport.feed` from the explicit
repository cwd. Only an explicit `--feed-script /absolute/user/driver.py` uses
the external extension loader (including that user's relative imports). It
cannot be combined with `--feed-driver`; no internal module uses the extension
loader. `VANLLIA_DIAGNOSE` supplies the absolute Kernel export path; qtest sets it
from its known root. Direct memory-driver callers must supply it explicitly.
Neither the source location nor capture-directory depth determines that input.

Without `--scene`, the health invocation deliberately retains the missing-image
`scene` sentinel. It must not be reported as an all-green suite, and this
migration does not hide it with an automatic filter.

Only `product` has a Vanllia image deployment today. All other scene names fail
before building, starting helpers, or creating evidence. The original scenario
branches/checkers are retained for subsequent provider intake; their presence
does not mean the corresponding images are available. There is no legacy image
builder fallback and no implicit mapping from `accept` to `product`.

The pending scene inventory is:

- `accept`, `system-fault`, `rig`, `load`, `group`, `beat`, `again`;
- `memory-transfer`, `memory-transfer-tlb`, `memory-transfer-fresh`,
  `memory-transfer-race`;
- `daemon-rtc`, `daemon-target`, `daemon-stop`;
- `iommu-probe`, `iommu-dma`, `iommu-product`, `iommu-product-fault`,
  `iommu-product-start-before`, `iommu-product-start-after`, `iommu-dual`,
  `iommu-dual-long`, `iommu-soak`, `iommu-soak-long`, `iommu-ruin`, `iommu-forced`,
  `iommu-fail-invalidate`, `iommu-fail-reset`, `iommu-fail-release`,
  `iommu-fail-reference`, `iommu-fail-fence`, `iommu-fail-readback`,
  `iommu-guard-pci`, `iommu-guard-iommu`, `iommu-guard-virtio-blk`.

`iommu-product-start-before/after` retain their original image-name mapping to
`iommu-product-before/after` in the frozen driver. Enabling these scenes requires
their actual deployments, image coordinates, fixture programs, feature flags and
provider identities, followed by new runtime evidence. Fresh/race/confirmation
scenes still require the original debug-assertion and Kernel-feature contracts;
no validator is relaxed here.

The standalone Shell idle driver takes `--root` explicitly and requires
`SHUYRA_KERNEL` from the build record, alongside `SHUYRA_INITRD` and `SHUYRA_SBI`:

```sh
python3 -m acceptance.drivers.shell.idle --root /absolute/vanllia
```

This starts a VM and requires runtime authorization and new guest evidence.
No idle result is claimed. See [ownership and coverage](../docs/acceptance-ownership.md)
for the comparison and the independent product-contract #8 boundary.
