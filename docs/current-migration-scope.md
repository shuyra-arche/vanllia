# Current migration scope

The user prioritizes current product code and builds, and expects destructive
changes. Old tests and scenes will not be ported merely to retain their count.
This supersedes the former sequence in `legacy-migration-order.md`; the frozen
source and completed work remain available as evidence.

| Decision | Concrete scope | Reason and coverage boundary |
|---|---|---|
| Keep | Product composition, 17 selected images, Kernel build, core service code and QEMU default-firmware selection | These serve the current product. The existing `cargo image`/`scripts/build.nu` entries remain the build authority; no multi-scene builder is added. |
| Keep | Remove `legacy-composition` from the root development graph; retain the completed #9 change | This removes an actual source dependency. The existing published commit and its tests are not rewritten or deleted. |
| Keep | Separate historical references from required inputs in the existing evidence tools; initialize only current provider paths | These are the remaining active tool/setup reads of the old repository. Historical content stays unverified; missing/dirty/wrong current providers still fail. Only validation needed for this behavior is added. |
| Stop | Migration of the 18 frozen Rust suites, source-layout assertions, per-case parity expansion and coverage-count targets | Destructive changes would invalidate this work. Existing provider tests stay where they are; completed Daemon/Login/Router comparisons are retained externally, not copied into a new root suite. |
| Stop | Migration of all 35 old guest scenes and the central multi-scene CLI/registry work | Their existence is not a current product requirement. The proposed adapter changes were backed up and removed from the active worktree. The current entry still accepts only product/health for runtime. |
| Pause | New `daemon-rtc` Root/controller programs, scene declaration and static-contract suite | Source, two built ELFs and completed host/static checks are checkpointed separately. They are not integrated into the product or exposed by `cargo image`; no six-image initrd or live recovery result is claimed. |
| Defer until a concrete product need | Target/stop fixture APIs, supervisor stress harnesses, memory-transfer probes, PCI/IOMMU/virtio service extraction and broad accept | Add a focused check for the new implementation or a demonstrated failure. Do not preserve a second old runtime assembly solely for historical equivalence. |

The stopped scene families are explicit: Daemon (`daemon-rtc`, `daemon-target`,
`daemon-stop`); supervisor (`rig`, `load`, `group`, `beat`, `again`);
`system-fault`; memory (`memory-transfer`, `memory-transfer-tlb`,
`memory-transfer-fresh`, `memory-transfer-race`); the 21 `iommu-*` scenes; and
`accept`. The exact 35-name frozen mapping stays in `legacy-scenes.json`.

This loses no current product image or core service implementation. It defers
new proof of live RTC restart, supervised stop/target behavior, stress and
memory-race/translation behavior, device failure/recovery and the broad accept
deployment. Existing host checks are not substitutes for those runtime proofs.
No QEMU/SMP execution or supported-runtime claim is made.

The actual code/build distinction matters: candidate #9 at
`bba900159c44b9f7df98b5250c8bdea320ca58f8` already built without sqware source
or caches. Main was `7e284f600b919d9e19fcdae6b96352e1c8a4ef79` at this
checkpoint and still retained the old development edge. Reviewing/converging
that existing PR is the next mainline step; porting old suites/scenes is not
its prerequisite. This local follow-up additionally removes the old checkout
requirement from schema-2 evidence generation. It does not merge or publish.

Preserved local checkpoints are `05db64f263d36f70accde7654c8a3f79b32c7a47`
(one provider-coverage note) and `a085887bf94d8c514f78320abdc8b8d9e600e106`
(paused fixture workspace). Neither is an ancestor of this active outcome.
Raw results and verified recovery copies live under
`/workspace/boot-review/parallel-next-batch/`. The current tree no longer
registers the `legacy/sqware` gitlink or its `.gitmodules` entry. The frozen
revision `94e114614bec53424ac55e640fc80cf19a3e7107`, prior commits and records
remain historical evidence. Existing local source and its Git directory are
preserved; `/legacy/sqware/` is ignored exactly so it does not dirty the current
checkout. No archive or source-data deletion is part of this change.
