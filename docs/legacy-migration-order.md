# Remaining legacy ownership and migration order

This is a source/host-plan inventory, not an enablement or runtime result.
It uses frozen sqware `94e114614bec53424ac55e640fc80cf19a3e7107`, selected
planner `17fd7ddbd18b3aef3a8d348cd32d8642e81b3c7f`, and Vanllia product code
at `903a6cad7c70350ddbf0ed9618bd96a55c7f7270`. The external host inventory
loads the actual published `DEPLOYMENTS`, `PROGRAMS` and `IMAGE_BUILDS`; it does
not compile or execute any guest fixture.

All 36 frozen deployments resolve through that planner. One is `product`,
already available in Vanllia; 35 entry scenes remain disabled. The complete
[scene/image inventory](legacy-scenes.json) records their actual image lists,
entry programs and additional images relative to the current product's 17.
`iommu-product-start-before/after` map to frozen
`iommu-product-before/after`; these names must not be conflated. No gate in
`scripts/qtest.nu` changes here.
The JSON is a recorded inventory, not a self-verifying test or runtime registry.
Raw host inventory, crate/source-reference reports, build logs and license
records remain external evidence; the frozen source and planner revisions above
are the inputs needed to regenerate the scene plan.

## Remaining source dependencies

The selected 17 production image inputs are independent of sqware; the root
dev edge `tools/product -> legacy-composition` still fetches four frozen sqware
packages: composition, Shell declaration, RTC API and fixturetarget API.
The five new literal product contracts do not automatically replace all six
retained equivalence cases. Preserve their checks until each successor assertion
and negative sensitivity is mapped, including owner descriptors, namespaces,
registry slots, catalogue ports and bindings. Do not copy a second complete
legacy registry into production to remove the dev edge.

The physical `legacy/sqware` gitlink, `.gitmodules` entry and workspace exclusion
also remain. Removing a Cargo edge and removing this checkout are separate
steps. Historical references, credentials and the Kernel's
`sqware-diagnose.jsonl` protocol filename are not source dependencies and must
not be renamed as a substitute for migration.

There are 18 old Rust test crates under `programs/tests`:

| Responsibility to map | Existing crates |
|---|---|
| Cross-owner wire/declaration and System contracts | control-api, control-rpc, publication-admission, interface-marks, system-shape |
| Session and Router behavior | login, router-handoff |
| Shell behavior/adapters | shell, shell-adapter |
| Daemon recovery policy | daemon-runtime |
| Device protocols and driver behavior | device-api, device-recovery, iommu-pages, iommu-queue, pci-placement, pci-scan, virtio-blk-coord, virtio-caps |

Static inspection finds 115 literal Rust `#[path]` references across these
crates, all resolving in the frozen checkout. This is not their complete import
closure: tests such as system-shape also read source files dynamically. No test
was retired and these 18 suites were not rerun here. Provider-green results
alone do not prove that their assertions or real source closures were preserved.
Keep new tests with the owning responsibility; cross-owner contracts stay with
integration, and host fixtures stay separate from runtime/driver code.

## Minimum dependency order

Treat each row as a cohesive reviewable outcome, not a PR per scene or crate.
Within a row, checkpoints remain on one branch until its assertions, providers,
image builds and integration contracts can be reviewed together.

| Order | Outcome and smallest first slice | Completion boundary |
|---|---|---|
| 1 | Map the six product equivalence cases to independent expected data/contracts, then migrate only the host assertions whose owners are already providers. | Preserve all assertions and import/source-closure evidence before dropping the frozen root dev edge; do not delete old suites by directory name. |
| 2 | Daemon scene family: begin with `daemon-rtc` (two additional images), then target/stop together (five distinct additional images, sharing fixturetarget). | Daemon owns policy and its host cases; test-support owns guest entry/controller images and fixturetarget API. Reuse current Hub/Router/RTC/Daemon providers and preserve `scene`/`supervised` membership and final cleanup checks. |
| 3 | Small supervisor fixtures: rig/load/group/beat/again (ten distinct additional images), then system-fault (four). | Separate actual entry/resources from reusable mechanism; publish explicit image coordinates without importing the old programs aggregate. Preserve source-derived failure/termination behavior. |
| 4 | Memory-transfer family: basic pair, TLB entry, then fresh and race entries/peer (six distinct additional images total). | Retain pinned Kernel test API, diagnostic input and streaming/final predicates. Fresh/race require their Kernel feature and debug-assertion contracts; current scene runner forces a release test target, so changing only the scene gate is insufficient. |
| 5 | Device chain: probe/DMA entry fixtures first; establish PCI/IOMMU/virtio-blk server ownership before product/device-recovery variants. | At the selected immutable pins, PCI provides API/declaration and IOMMU API/declaration/hold, with no server Cargo manifest. Independent server inputs are real missing work. Preserve NBD/hold/control helpers, two-device generations, fault injection, confirmation features and guard stop semantics. |
| 6 | Broad `accept` deployment last: 39 images, 24 additional to current product. | All guest/namespace/probe owners and assertions must be accounted for; retain its own accept-system entry. Never alias accept to product. |

Daemon RTC uses six total images, target and stop four each. System-fault uses
17; memory basic/race two each and TLB/fresh one each. Basic device probe/DMA
plans each contain one image; product device variants contain 22, recovery
variants seven. These are frozen plan facts, not proof that those guest programs
already have independent buildable inputs.

The current acceptance package and 320 host cases retain the reorganized owners.
Their presence is useful validation infrastructure, not a claim that disabled
guest scenes are migrated. No default health sentinel is hidden or filtered.

Remove the old checkout only after every required fixture/test source closure
and image deployment is independent, followed by a checkout/build verification
without it. New runtime evidence and firmware selection/distribution decisions
remain separate archive gates. Nothing in this plan authorizes QEMU, SMP,
publication, branch deletion, archive or retrying the previously denied merge.
