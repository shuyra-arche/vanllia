# Acceptance material

`tests/` contains the 55 Python host drivers, checkers, synthetic fixtures and
unit tests moved from `programs/tests/` in
[`anreonyr/sqware` at `94e11461`](https://github.com/anreonyr/sqware/tree/94e114614bec53424ac55e640fc80cf19a3e7107/programs/tests).
The directory depth is preserved so repository-relative trace lookups and
sibling imports still work. The source manifest in `source-files.json` records
each original path and Git blob, including the four migrated Nushell scripts.
It links the earlier history without requiring the old checkout at test time.
No original copyright or license notice is removed or replaced.

Run the host suites separately because the legacy directories use sibling
module imports and share names such as `trace`:

```sh
python3 -m unittest discover -s scripts/tests -v
python3 -m unittest discover -s acceptance/tests/device-recovery -v
python3 -m unittest discover -s acceptance/tests/daemon-recovery -v
python3 -m unittest discover -s acceptance/tests/memory-transfer -v
```

These commands use synthetic data, local sockets and executable stubs. They do
not boot QEMU. Do not treat a Python syntax/import check as runtime acceptance.
Files named `idle.py`, `session.py`, `smoke.py` and other driver entry points
can start QEMU or interact with a guest when run directly; those commands are
runtime work and are not part of the host checks above.

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

The standalone Shell idle driver now requires `SHUYRA_KERNEL` from the build
record, alongside `SHUYRA_INITRD` and `SHUYRA_SBI`. This replaces its hard-coded
old Kernel target path. The other 54 Python files remain byte-identical to the
recorded source blobs. The driver still requires runtime authorization and new
guest evidence; no idle result is claimed here.
