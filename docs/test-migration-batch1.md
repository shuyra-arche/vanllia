# First host-test migration: namespace admission

This records the first checkpoint at `03b805ad8ed25f9f0e59203e64985a985b477433`.
Its six-comparison/15-case totals are historical; see the
[next migration map](test-migration-batch2.md) for current retirement status.

Base: `31af57ebe0d639f99fa17ca5514df34343604437`.
Frozen source: sqware `94e114614bec53424ac55e640fc80cf19a3e7107`,
`programs/tests/publication-admission/lib.rs`, Git blob
`f4fa7f1310fcaaed61f7cccaeb8e03957deada74`.

This batch migrates one complete host contract to
`composition/tests/namespace_admission.rs`. Composition owns the namespace
policy supplied to System; the test consumes the selected public Unit/System
types. A local `shared_namespace` fixture supplies explicit policy inputs. It
does not reproduce namespace validation, load a source file, inject modules or
import the old test crate. Flat Cargo integration tests replace the old
standalone test-crate wrapper; no old directory structure is copied.

## Assertions and product behavior

The first three test names retain their frozen identities:

| Case | Retained static assertion macro sites | Behavior |
|---|---:|---|
| `composition_controls_layout_and_bounded_names_without_service_classes` | 6 | Valid declaration/path, allowed finite name, rejection of unlisted/traversal names and invalid declaration path. |
| `finite_grants_can_narrow_but_cannot_escape_the_installed_scope` | 3 | Finite subset accepted; outside name and expansion to subtree rejected. |
| `permission_choice_is_explicit_and_shared_base_requires_no_extra_directory` | 4 | All four permit choices retained, shared base accepted, empty finite scope rejected. |

All 13 static assertion macro sites, the complete three case bodies (including
loops) and the fixture body were compared using a one-time Rust syntax/token
audit. Both files are parsed as Rust; the comparison is normalized token-tree
equality, not a general semantic-equivalence proof or a runtime assertion count.
Only whitespace, optional trailing list commas, `rules -> shared_namespace`
and the old helper's `super::declaration::` prefix were normalized. The API
identities below establish why that import-prefix change is valid.
The baseline's `alloc::vec!` spelling is retained for that comparison; these are
explicitly host tests, not freestanding test targets.

The additional `registered_product_namespace_policies_satisfy_admission` case
checks the actual published `PROGRAMS` namespace declarations with the selected
admission API and rejects a completely empty policy iteration. It does not
freeze each program's namespace count or replace the frozen owner-field cases.

The permit loop preserves representation and fixture policy choice; it does
not prove runtime permission enforcement. This batch does not exercise live
identity arbitration, publication traffic, Kernel behavior or guest cleanup.

## Dependency correspondence

| Old dependency | Existing composition dependency | Immutable revision |
|---|---|---|
| normal `env` | normal `env` | `d3000253cd5d13d466e91fafd4dbb27a5ec5aaae` |
| normal `system-api` | normal `system-api` | `9e5d63ca536a176583cf1b0054b4ecf7ddd88257` |
| dev `unit-declarations` (package `unit`) | normal `unit` | `90396f0fda376e0693fb1b9553aa21864d241057` |

No dependency, manifest, provider pin or lock changes. Root lock remains 95
records. The selected composition normal/development/build closure resolves
without sqware packages. The workspace still has the separate image-tool
`legacy-composition` dev edge and Cargo may resolve its frozen Git sources;
neither whole-workspace dependency removal nor empty-cache independence is
claimed.

## Current product equivalence mapping and remaining gaps

Every existing equivalence case remains byte-identical and runs in the combined
host gate. The pre-existing five literal product contracts cover two comparison
responsibilities; this batch adds a different namespace policy responsibility.

| Retained equivalence case | Current independent coverage | Still requires frozen comparison |
|---|---|---|
| `product_deployment_fields_match_pr55` | Existing deployment fields and target/bootstrap literal contracts. | No retirement performed; its old-source comparison remains. |
| `product_inventory_and_startup_match_pr55` | Existing coordinate-name order, typed inventory and startup literal contracts. | No retirement performed; no claim that every build descriptor is frozen by those literals. |
| `provider_units_and_build_coordinates_keep_frozen_fields` | Existing coordinate name/count contract; new declaration admission check is complementary. | Daemon ownership/fields, per-image descriptor/routing, uniqueness and zero selected legacy manifests. |
| `selected_owner_declarations_keep_registry_slots_and_frozen_fields` | New admission check covers validity of the published policies. | Login/UART/RTC slots, pointers, namespace/loader/control/identity fields and build coordinates. |
| `shell_program_keeps_registry_slot_and_frozen_source_fields` | New admission check covers validity of the published Shell policies. | Shell registry ownership, complete observable fields, provider descriptors and old Shell/Cat transition fixtures. |
| `shell_command_catalogue_keeps_frozen_ports_and_bindings` | No replacement in this batch. | Catalogue ordering, ports, bindings and command-image correspondence. |

All six retained cases still use frozen composition. Four responsibilities do
not yet have complete independent successors; the suite keeps frozen
composition, Shell declaration, RTC API and fixturetarget API reachable through
the image tool's dev fixture. Removing that edge needs assertion-by-assertion successors, not
removing a test directory or replacing old declarations with another runtime
registry. The selected batch is fully mapped. No migration/retirement claim is
made here for the other 17 old Rust test crates: their assertion/source-closure
mapping remains open. All 35 guest-scene gates remain unchanged. Even the frozen
original publication-admission crate is retained as history/baseline, not deleted.

## Verification

Linux x86_64, repository `nightly-2026-09-04`:

```sh
cargo test --locked --offline -p vanllia-composition \
  --test namespace_admission --target x86_64-unknown-linux-gnu
cargo test --locked --offline -p vanllia-composition -p vanllia-image \
  --tests --target x86_64-unknown-linux-gnu
```

Frozen standalone baseline: 3 pass. New contract: 4 pass. Combined current host
gate: 15 pass (4 namespace, 5 previous contracts, 6 retained equivalence).
An independent checkout without initialized legacy/provider submodules also
passes all four new cases with the pre-existing Git/dependency cache available.
The 320 Python tests are unchanged and were not rerun for this Rust-only batch.

Twelve isolated test-input variants produce the intended case's assertion
failure, never a compilation failure: malformed fixture path, empty installed
names, missing allowed name, replacing an invalid path with a valid one,
broadened parent scope, replacing an outside child name with an inside name,
replacing the subtree request with a finite one, discarded permit, invalid
shared base, valid replacement for the empty scope, invalidated local copies of
actual registry policies, and empty actual registry iteration. The copied-policy
variant changes each visited copy's path to `../escape`; its first reported
failure is Hub's service namespace. The immutable provider declarations are not
written. These demonstrate assertion/input
sensitivity, not API algorithm mutation coverage; provider implementations were
not altered.

An initial harness used ambiguous partial restoration and is invalid evidence.
Its logs are retained separately and not counted. The accepted run verifies
base bytes before every variant, restores the entire file, checks exact restored
bytes after every variant, and ends with a four-case passing control run.
Candidate source was never modified by the negative harness.

Raw baseline/new/negative logs, syntax audit and dependency/source evidence are
retained externally. [The machine-readable coverage record](test-migration-batch1.json)
records immutable inputs and per-case counts; it is provenance, not a replacement
for executable assertions. No QEMU, SMP, remote update, PR, archive or retry of
the previously denied merge is part of this batch.
