# Proposed versions, branch convergence and release gates

Status: **local proposal, not an approved version or permission to publish**.
Checked on 2026-11-05 against local candidate
`d3ac12fab06911fa8071837ed36f01eb313010fe`, tree
`d71c5621936f947921e5b876bc357ef7ca947b7a`.
This document and its read-only inventory helper follow that candidate; they
change no package version, dependency, gitlink, lock or acceptance gate.
The SHA/tree above identify the **input** before these two files are added,
not the tree containing this proposal. The generated inventory hash binds to
that input commit. The exact subsequent proposal head/tree belongs in its
external review/checkpoint record, avoiding a self-referential tree hash here.
Vanllia merge approval has not been obtained. The prior merge rejection remains
binding; changing the branch, PR target, merge method or tool cannot bypass it.

## Version responsibilities

Use independent component release versions and a separate Vanllia combination
version. Neither requires all repositories to share a number or release date.
The starting numbers, tag spelling, first supported compatibility surface and
release scope remain decisions for the user/maintainers. No number is selected
by this proposal. Existing `0.1.0` Cargo metadata is not evidence of an approved
component or Vanllia release. No Vanllia tag was advertised by the remote at
this checkpoint; component tag histories have not been surveyed.

| Owner | What its version should describe | When to prepare a release |
|---|---|---|
| Kernel | Declared Kernel ABI, image/boot contract and supported configuration | A reviewed change to that supported contract or its implementation |
| System, Shell, Daemon, Pipe and individual Driver/Session provider repositories | The owner's documented API/IPC/declaration/build-descriptor contract and behavior | A coherent owner change with its host tests, required consumer checks and change notes |
| Driver and Session clusters | A tested selection of their independently owned providers and exposed aggregation API | A provider selection or aggregation change; member versions remain independent |
| Image builder | Host declaration/planner/build interface and artifact-discovery contract | A reviewed host behavior or interface change |
| Shared SDK/API repositories | Explicit wire, identity and type compatibility contracts | A reviewed API/ABI or compatible implementation change |
| Vanllia | A supported, tested combination of exact owner sources, tools, product declarations and configuration | A combination passes the gates for its explicitly stated release scope |

Declare the public contract before applying Semantic Versioning. For a stable
contract, incompatible changes, compatible additions and compatible fixes have
different version increments. For a pre-1.0 series, explicitly document the
chosen compatibility policy; a `0.x` label alone promises no stable public API.
These are proposed rules, not a decision to adopt a particular first number.
See the [SemVer specification](https://semver.org/spec/v2.0.0.html).

A repository release normally names one source commit and lists its package
versions. It need not create a separate release for every test crate. If public
crates are independently published, record each crate's own version and source
mapping instead of pretending a repository label changes Cargo package IDs.
An upstream release does not automatically trigger a Vanllia release. A
component can advance while Vanllia retains its previously accepted pin.

## Build facts stay authoritative

Human version labels name releases. Full commits, recursive gitlinks,
manifest dependency declarations and Cargo locks identify the actual sources.
Keep the current full `rev` pins and committed locks; introducing version
labels is not a reason to replace them with floating branches or tag lookups.
Cargo distinguishes dependency requirements from the exact resolution in a
[lockfile](https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html).
The generator below inventories existing lock bytes; it never edits them.

The inspected top-level selections are:

| Checkout | Full gitlink |
|---|---|
| `components/kernel` | `3aa0a155f825fbf377fc8d04009472ca67cbbf8c` |
| `components/system` | `9001af65f97501dfa490c9397c61d593c871136d` |
| `components/image-builder` | `17fd7ddbd18b3aef3a8d348cd32d8642e81b3c7f` |
| `components/driver` | `bb3427e179e87b640e0d51115a44544fae231252` |
| `components/session` | `87abc22d08ffe311b7e43d915240d318642d510c` |
| `components/pipe` | `4fce76372beb6c5ab8567232516f156b3d1032eb` |
| `components/daemon` | `4e4ae06ee488e9f02e15d74405b989f20a5a286f` |
| `components/shell` | `5d84650d37df7523d0ac59fc563e2fc7dc4025ba` |
| `legacy/sqware` — historical/dev fixture | `94e114614bec53424ac55e640fc80cf19a3e7107` |

The root lock has 95 package records, blob
`63c614ed6e3a4892cda1f0b06c41067da644a65f`, SHA-256
`2e5567fa5ce6f3b29a97b07e907c6dc7b68d9615af3868fdda444f2734a2f394`.
It includes development fixtures and is not the production dependency graph.
Keep separate per-consumer, per-target normal/build/dev graph reports, including
features and package source IDs. A path package ID is tied to its checkout's
verified gitlink; it is not automatically the same Cargo identity as a Git
package with identical source bytes.

One repository URL can legitimately appear at multiple selected revisions.
For example, System server is `9001af6...`, bootstrap support is `5a792ff...`
and System API is `9e5d63c...`. The inventory records those package-specific
sources separately. It must not choose the checkout SHA for all three or claim
one future System tag describes all three source snapshots. Any consolidation
is a later reviewed dependency change with consumer compatibility checks.
Enforce required SDK identity uniqueness inside each actual consumer closure,
not a fabricated rule that every URL has one revision across all workspaces.

## Converge existing review work before adding branches

The live remote `main` is
`9edc868ead5a765d23eb1ff645285fc476f92dbf`. The following PRs remain open,
Draft and unmerged. Their live heads and bases were read from GitHub, and all
five head commits are ancestors of the local candidate:

| PR | Head | Current base branch |
|---|---|---|
| [#4](https://github.com/shuyra-arche/vanllia/pull/4) | `8286a865d97e22ba19edb0182dfe209c607badb5` | `main` |
| [#5](https://github.com/shuyra-arche/vanllia/pull/5) | `2643ee792b9fc791e24bbfc3a8efedc1f83fd69a` | `codex/shell-command-images` |
| [#6](https://github.com/shuyra-arche/vanllia/pull/6) | `3003ee3abc8d18be3673e6df7abf17c58076d5fa` | `codex/login-uart-rtc-intake` |
| [#7](https://github.com/shuyra-arche/vanllia/pull/7) | `9d540b3e42f27b7904eafee068b3d094e7608eb5` | `codex/login-uart-rtc-intake` |
| [#8](https://github.com/shuyra-arche/vanllia/pull/8) | `5ebafc855980cf6ff7115d4a9ae6980d9b76dd38` | `codex/login-uart-rtc-intake` |

The local `integration/vanllia-6-7-8` branch already contains #4 through #8.
It retains true merge commits `b25d7b0...` (#6+#7) and `ca7c4ef...` (+#8),
then build/firmware evidence and two host-test migration batches. Its name is
historical, not a requirement to rename it or make another branch. Ancestry
proves preservation of commits; retained source checks and coverage mappings
are still required to assess the final tree.
The rechecked parent relationships are:
`b25d7b0f5ed6af612ae8e84473c86c2c2fde9f5e` joins #6/#7;
`ca7c4ef584c6337065686fb7ddf48f421b0243aa` joins that merge/#8; and
`29d51e2c0406b6cf3b03e2344ae941089626e703` directly follows it. These
relationships were rechecked from local Git objects for this proposal.

Proposed convergence, **only after the relevant remote actions are explicitly
authorized**:

1. Freeze one complete local outcome and its exact reviewed head/tree. Re-read
   main and all five PR heads, check for other writers and require the reviewed
   #6 head to remain the expected ancestor. If anything changed, reconcile and
   review locally before proposing publication.
2. Reuse existing Draft #6 as the single combination review. Its branch can
   advance normally to this candidate because #6 is already an ancestor.
   Retarget that same PR to `main` so its diff includes #4/#5 as well as #7/#8
   and the follow-ups. Update its description around the final outcome and
   validation. No new roll-up PR, force push, squash or rewritten history is
   needed. This is a proposed future publication, not performed here.
3. Review that exact combined diff against live main. Explicit Vanllia merge
   confirmation is a separate prerequisite; local tests or this plan do not
   supply it. A future authorized merge must preserve the constituent history.
   If repository settings prevent that, stop and resolve the policy rather
   than switching methods to bypass the restriction.
4. After an authorized merge, verify each frozen PR head is an ancestor of the
   resulting main and that the approved tree/assertions/pins survived. Only then
   reconcile the status of #4/#5/#7/#8 and their review records. GitHub is not
   assumed to close every stacked PR automatically. Closing PRs or deleting
   branches needs its own applicable authorization; none is done now.

For future work, use `main` plus one bounded branch/PR per independently
reviewable outcome in the owning repository. Keep intermediate checkpoints
on that branch. Update the existing Vanllia combination review when several
component changes belong to one outcome. Create a maintenance branch only for
an actually supported release line, not for every component or version bump.
After approval, use immutable release tags/records; never move an existing
release tag to repair an artifact. A repair gets a new version and evidence.

## Generate the combination inputs, then attach build and acceptance records

[generate-composition-inputs.py](generate-composition-inputs.py) uses Python
3.11+ and Git supporting `--no-lazy-fetch` (validated with Git 2.52.0). It reads
committed objects at a required full SHA, follows
initialized production-side recursive gitlinks and records their exact commits,
trees, manifest/package metadata, lock hashes/counts and toolchain/config hashes.
It records the legacy checkout at its frozen top level; its child gitlinks are
listed but not expanded. This boundary is explicit and does not remove it.

```sh
candidate_sha=$(git rev-parse HEAD)
record_dir=$(mktemp -d)
python3 docs/generate-composition-inputs.py --commit "$candidate_sha" \
  --output "$record_dir/composition-inputs.json"
```

The helper does not fetch, invoke Cargo, resolve tags, run any guest, or read
credentials. It fails on an unavailable required submodule/object and refuses
to overwrite an output file. Run setup separately in a chosen checkout; this
helper does not prove an empty-cache setup works. Worktree edits are not
inventoried: the source is the exact commit, even if HEAD/worktree differs.
Branch/tag names are rejected. The same object inputs produce the same JSON.
Git calls disable lazy fetching, replacement objects and optional locks; see
the [Git options](https://git-scm.com/docs/git#Documentation/git.txt---no-lazy-fetch).
Ambiguous duplicate submodule paths are rejected rather than silently choosing
one URL. An independently cloned source directory is usable if it contains the
exact gitlink object; its current checkout HEAD is not the source authority.

At this checkpoint the inventory expands 18 repository checkouts, including
Vanllia and the legacy top level. It records 102 tracked locks across those
trees, including test/integration locks; this is **not** 102 selected production
builds. Only the root lock's Git package identities are expanded in JSON; the
other locks are recoverable by repository commit, path and blob/hash. Some
checkout paths intentionally select the same Kernel commit. Repeated paths or
historical/dev identities must not be silently counted as new runtime owners.
The 18 figure counts checkout paths. The root lock's 43 Git package records
count package identities, not 43 repositories or checkout paths: multiple
packages can share a source, and a repository can supply several revisions.
The 102 figure counts tracked files. None of these counts describes a selected
production closure or a complete release manifest.

Output has `kind: committed-input-inventory`, `schema_version: 1`, an explicit
Vanllia source commit, and null Vanllia/component release versions. It leaves
`release_approved: false`, `build_records: []`, `firmware_selection: null` and
`runtime_evidence: []`. These are honest missing facts, not release defaults.
`is_release_manifest` and `is_dependency_graph` are both explicitly false.
Save generated records outside the source commit (or as future authorized
release attachments), avoiding a self-referential manifest commit hash.

A complete release record must bind the input inventory's content hash to:

| Record | Required data and source |
|---|---|
| Version decisions | Chosen Vanllia version; independently chosen component versions or explicitly unreleased source status; exact tag objects/commits if tags are used; scope and compatibility notes |
| Selected consumers | Actual Cargo metadata/normal/build/dev reports per manifest, target and features, canonical package source IDs and verified local source mappings |
| Build invocation | Root/source commit, toolchain and safe tool versions, target/profile/features and flags; the real `scripts/build.nu` JSON output and logs, not an assumed old path |
| Product contents | Actual ordered 17-image inventory, kinds, complete build coordinates and nine-entry startup; packed ELF/parser checks and artifact sizes/hashes |
| Kernel and firmware | Kernel artifact/hash and configuration; explicit firmware artifact/hash, upstream commit, resolved config, toolchain and applicable source/license/notice materials |
| Acceptance | Exact source/config/artifact hashes, command, environment, expected assertions, raw logs and verdict; distinguish passed, failed and not run |
| Review and release | Exact approved tree, outstanding exclusions and explicit publication/merge/release approvals; never infer these from a model review |

The inventory is not a selected dependency graph, SBOM, build certificate,
license clearance, or runtime result. Those records require their own evidence.
Independent [build/host-test observations](execution-evidence.md) can now be
generated by `scripts/record-evidence.py` for an exact clean input checkout.
They bind the inventory hash to actual execution, tools, logs and reported
artifact hashes. The inventory's empty evidence arrays remain unchanged;
records are separate. A read-only [group consistency check](evidence-group.md)
can require all three actions and detect mixed inputs/configuration or changed
retained files. A full release-record packager and gate evaluator are not
implemented, and a consistent group is not release approval.
For registry dependencies, the committed lock bytes retain checksums even
though this compact inventory expands only Git identities from the root lock.
For component source retention, confirm pinned objects remain retrievable
before removing branches or archiving an old source; a remembered SHA is not
proof of future availability.

## Gates and current evidence boundaries

| Gate | Requirement before claiming completion | Current checkpoint |
|---|---|---|
| Coherent review candidate | Exact combined diff, constituent history, coverage mapping, immutable pins and no unexpected changes | Local combined candidate exists; no combined remote update or main merge |
| Host contracts | 320 acceptance host cases and current Rust host gate, locked dependencies; preserve old assertion successors and investigate relevant negative failures | 320 passed at integration commit `ca7c4ef...`; acceptance/tests/scripts are byte-identical at the input commit and untouched by this proposal. Rust 17 and isolated composition 14 passed for batch-two code, with 37 intended negative failures and zero compilation failures. Not rerun for this documentation/helper change |
| Build preparation | Actual product+Kernel release entry, 17 packed images and actual parser/static checks; retain full commands and hashes | Passed at `29d51e2...`, recorded in [build evidence](integration-build-validation.md). Current production sources/pins match; the later root lock adds only a test edge. No new exact-final-head build or empty-cache reproducibility claim |
| Firmware selection/distribution | Explicit reviewed artifact/config and applicable source/notices before using or shipping it | Old SBI origin remains unknown. Official OpenSBI and optional reduced config are build evidence/proposals, not a selected or legally cleared shipped artifact; see [firmware](firmware.md) and [configuration](../firmware/opensbi/README.md) |
| Product runtime scope | Separately authorized QEMU/guest/SMP tests for the named configuration and claims, bound to the exact inputs | Not run for this candidate. Host stubs, static ELF checks and older PR1 SMP8 evidence do not satisfy it |
| Supported release | User-approved version/scope, reviewed combined tree, passing applicable gates, explicit release authority and complete records | No chosen version, tag, release, publication or merge approval |
| Whole-repository independence/archive | Finish or explicitly account for every required old fixture/test/scene source closure; verify checkout/build without old sources and satisfy agreed runtime/firmware requirements | Not ready: three declaration comparisons/five UnitFiles and four sqware dev package identities remain; 35 scene gates remain disabled. Follow [migration order](legacy-migration-order.md) |

The 37 batch-two negatives include 26 actual composition-input changes and 11
local-copy/slice/output changes; they do not establish provider or planner
algorithm mutation coverage. Cached Git/dependency objects were used for the
isolated host checkout. The earlier 320 host cases do not exercise real QEMU.
Historical records keep their original heads/counts; this table does not
retroactively reinterpret earlier test runs.

A product-only preview could propose an explicit smaller support scope, but
the user must approve that scope and its unmet gates must remain visible. It
cannot imply that all 35 old scenes were migrated or that sqware can be archived.
The immediate executable next step is review of this version/branch proposal,
then agreement on the release scope and version policy. It is not a command to
merge, tag, publish, delete branches or archive a repository.
