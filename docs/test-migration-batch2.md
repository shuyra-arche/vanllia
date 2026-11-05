# Product plan, image and Shell resource contracts

Base: `03b805ad8ed25f9f0e59203e64985a985b477433`; old equivalence blob
`2f0d5bc7a529775b3dbfd294b369a68e5ff6c93e`.
This batch targets the comparisons that directly read old deployment, image and
Shell catalogue data. It does not move another standalone API test crate.

Composition now owns five additional host cases in two responsibility files:

- `image_coordinates.rs`: all eight fields of each of the 17 ordered Cargo
  coordinates, plus actual owner descriptor fields, one selected route per
  owner and provider-relative manifest prefixing;
- `shell_catalogue.rs`: the six ordered command names and typed port directions,
  complete command/aggregate/program supply bindings, and their relationship to
  the published product's resources and selected image bins.

The small literal matrices are test-only expected data, not another runtime
registry. No old source, legacy crate or generated observation is imported by
these tests. Expected coordinate values combine frozen source with the reviewed
provider intakes already present in base code, not the current producer's output
at test execution time.

## Frozen inputs and correspondence

Frozen sqware source remains `94e114614bec53424ac55e640fc80cf19a3e7107`:

| Source | Git blob |
|---|---|
| Shell `declaration/src/commands.rs` | `b1c7e1aa6f7104d9d145935863439f2a96a53d4c` |
| Composition `src/image_builds.rs` | `5564c0ffa4f4b610dac16f8426f2fa930a59a0be` |

The selected Shell provider is `5d84650d37df7523d0ac59fc563e2fc7dc4025ba`.
Its command declaration bytes equal the frozen command file. Command ports,
source/resource keys, slot marks and FETCH permissions are literal expectations
from that frozen source. These use the selected public API encoders and do not
claim independent raw-key/hash algorithm validation.

Coordinate expectations retain the approved provider transition: new System
bootstrap; Login/UART/RTC/Daemon providers; and Shell/Cat plus five commands from
the ordered Shell aggregate. Vanllia #7 `9d540b3...`, its immutable provider
descriptors and the already-recorded combined release build supply the changed
package/bin/manifest facts. Names, target/profile, features and default-feature
selection retain the original observable coordinate contracts. This batch
changes none of those production inputs.

The old six-case gate passed before retirement. The replacements also passed
while the old cases were still available. The mapping is by behavior, not by
case-count equality:

| Old case / static assertion sites | Successor or retained responsibility |
|---|---|
| Deployment fields / 10 | Existing two literal deployment cases preserve name/entry, complete start/guarded/resources/sources, target count/names/members and ordered typed bootstrap. The old comparison is retired. |
| Inventory/startup / 7 | Existing coordinate-name, typed inventory and startup literal cases preserve 17 names/kinds and nine startup entries; the new complete-coordinate case strengthens descriptor coverage. The old comparison is retired. |
| Provider units/build coordinates / 40 | The entire Daemon field/pointer prefix remains as `daemon_declaration_fields_match_pr55`. All coordinate, count, uniqueness, prefix and no-selected-legacy-route checks move to the two image cases. |
| Selected owner declarations / 6 | Registry list, pointer, uniqueness and complete observable-field helper remain. Only the two coordinate statements move to owner-route and literal-coordinate contracts. |
| Shell program/source fields / 16 | Pointer/observable-field prefix and both literal legacy name/image guards remain. Aggregate/BUILD fields, selected prefix/bin and Shell/Cat transition checks move to image/resource contracts. |
| Shell catalogue / 9 | Typed ordered ports, literal complete bindings and resource/image cases preserve every current catalogue/bin condition. The old comparison is retired. |

The old Shell/Cat loop also checked the old fixture itself: package `programs`,
manifest `programs/Cargo.toml`, bins `prog-shell`/`prog-cat`. Those immutable oracle
facts were verified in the passing old gate and remain anchored to the frozen
source blob. They are not recreated as tautological assertions on constants.
The new selected routes keep those bins and use explicit different owner
packages/manifests. Removing repeated frozen-oracle self-checks is distinct from
removing a current product behavior check.

A Rust syntax/token audit confirms the retained Daemon prefix, complete shared
observable-field helper, selected-owner body except the two moved statements,
and Shell prefix plus literal guards. It permits whitespace/optional list
trailing commas, not arbitrary assertion changes. The retained-field audit is
not a general semantic-equivalence proof; the migration map, frozen inputs and
negative checks support the moved responsibilities.

## Dependencies and remaining removal boundary

Typed port expectations add one composition **dev edge** to the already-locked
`pipe-api` at `4fce76372beb6c5ab8567232516f156b3d1032eb`. The source is the same
API used by both frozen and current command declarations. All 95 package
identities, versions, sources and checksums stay unchanged; the only lock
dependency delta is `vanllia-composition + pipe-api`. No provider pin or normal
dependency declaration changes.

The selected composition normal graph remains exactly the same 33 packages and
edges. Its all-kind host-test closure has 65 packages and no sqware package.
The workspace image tool still retains `legacy-composition`; Cargo may resolve
that frozen Git source even when the composition tests are selected.

There are now **three**, not six, old equivalence cases. Their only legacy value
reads are five UnitFiles: Daemon, Login, UART, RTC and Shell. Root product tests
no longer directly compare legacy deployment/image tables or command catalogue/
binding arrays. Those source modules still compile as part of the frozen crate;
the source closure has not been removed. The frozen composition still brings
its four sqware package identities, so this
batch does **not** remove the old dev dependency or reduce that package count.
Next removal work is an independent expectation map for the remaining declared
loader, namespace, identity, control and supply fields. Never copy an entire old
runtime registry or drop those field checks just to eliminate the edge.

## Validation

Linux x86_64, `nightly-2026-09-04`:

```sh
cargo test --locked --offline -p vanllia-composition -p vanllia-image \
  --tests --target x86_64-unknown-linux-gnu
```

The current gate passes 17 cases: namespace 4, original literal contracts 5,
image coordinates 2, Shell resources 3, remaining frozen declaration cases 3.
An isolated checkout with no initialized provider or legacy submodules passes
all 14 composition cases, using the pre-existing Git/dependency cache.

Thirty-seven negative variants produce the intended test failure with zero
compilation failures. Twenty-six change actual composition declaration inputs;
eleven change local copies/slices of selected provider values or a planner
result. They cover every coordinate field, missing/duplicate images, owner
prefix/fields, command names/ports/directions, binding source/resource/slot/
permission, aggregate/program supply lengths, resource/bin membership, all
deployment field groups, inventory kind and startup comparison sensitivity.
They do not mutate provider or planner algorithms and do not prove algorithm
mutation coverage.

Every variant starts from verified candidate bytes. The whole mutated file is
restored, all mutable files are compared with the candidate after every case,
and a restored 14-case positive control passes. No candidate source or shared
Git cache was edited by this harness. Raw logs, input deltas, dependency audit,
frozen-field token audit and source fingerprints remain external evidence.
[The coverage record](test-migration-batch2.json) preserves the checkpoint inputs
and outcomes; executable assertions remain the validation authority.

Existing namespace and original five contract sources are unchanged. The 320
Python cases and 35 guest-scene gates are unchanged and were not rerun here.
No product/Kernel build, firmware selection, QEMU, SMP, remote write, new PR,
merge retry or archive operation occurs in this batch.
