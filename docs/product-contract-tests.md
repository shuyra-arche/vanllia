# Product contract tests: first fixture slice

The product contract tests live with `vanllia-composition`. They describe the
product declaration and its plan using reviewed literal values. They do not
import the legacy composition, include legacy source files, read a legacy
checkout, or generate expected values when a test runs. No fixture framework or
new production interface is introduced.

This slice is based on Vanllia #5
`2643ee792b9fc791e24bbfc3a8efedc1f83fd69a`. It adds five host tests in two files:

- `composition/tests/product_deployment_contract.rs`: deployment fields, then
  ordered targets and typed bootstrap tasks.
- `composition/tests/product_inventory_contract.rs`: build-coordinate order,
  derived inventory with typed image kinds, then startup order.

The coordinate vector, derived inventory and startup vector are independent
expectations. In particular, the inventory begins with `daemon` and ends with
`system`, while the coordinates begin with `hub`. Startup begins with `account`.
No expected vector is calculated from another vector or from the current
product under test. Bootstrap and image kinds are compared semantically, rather
than using `Debug` output.

## Frozen inputs and provenance

The expected product declaration is frozen at
[`anreonyr/sqware` `94e114614bec53424ac55e640fc80cf19a3e7107`](https://github.com/anreonyr/sqware/tree/94e114614bec53424ac55e640fc80cf19a3e7107).
Its product-only values were reviewed against the retained equivalence tests.
An external, one-off host observation evaluated that frozen declaration using
the same selected planning API as those tests. The observation is evidence for
literal review; it is not a test dependency or an expectation updater.

| Input | Immutable identity |
|---|---|
| Frozen `crates/composition/src/deployments.rs` | Git blob `826de7a3486d2eeb1761c5f78b25c96d4f8f5d1d` |
| Frozen `crates/composition/src/image_builds.rs` | Git blob `5564c0ffa4f4b610dac16f8426f2fa930a59a0be` |
| Retained Vanllia `tools/product/tests/equivalence.rs` | Git blob `7c6cbdb8d7e714f841711e044738bdd88e3871b0` |
| Selected planner, `vanllia-build` | `shuyra-arche/image-builder` `17fd7ddbd18b3aef3a8d348cd32d8642e81b3c7f` |
| Declaration types, `unit` | `90396f0fda376e0693fb1b9553aa21864d241057` |
| Image kinds, `env` | `d3000253cd5d13d466e91fafd4dbb27a5ec5aaae` |
| Raw observation JSON SHA-256 | `7156f97ef78cb5cd6aebf7145f2f6c786317273874782b4a4e1490543864d7f4` |
| One-off emitter source SHA-256 | `256111d24fde4a3f39f07012d7f6f003fe422ee8c48d19d45362094878a5ae20` |

All 67 non-emitter package identities and checksums in the observation's
standalone lock are present in the original 92-package product lock. The
observation used the selected `17fd7ddb` planner, rather than substituting the
legacy checkout's different image-builder gitlink. Expected values are reviewed
data; changes require an explicit behavioral-contract review.

## Coverage and retained parity

Every existing test in `tools/product/tests/equivalence.rs` is byte-identical to
the base. Its SHA-256 remains
`47f07cf69296c90d46f63f5af21c933abff4f90cc72e2dd4ad08b391b84be93a`.
No old assertion is deleted, moved, relaxed or retired.

| Existing product comparison | New independent contract coverage |
|---|---|
| Deployment name and entry | `product_deployment_fields_are_frozen` compares `product` / `system` and also requires exactly one published product deployment. |
| Ordered start, guarded, ordered resources, sources | The same test pins the complete start/resource sequences and exact empty guarded/source slices. |
| Target count, ordered names and ordered members | `product_targets_and_bootstrap_are_frozen` pins both complete targets, including `scene` and `supervised`. |
| Bootstrap sequence | The same test pins the two typed variants in order: `Operator`, `Identity`. |
| Image-build count and ordered image names | `image_build_coordinate_order_is_frozen` pins all 17 image names and their order; it does not freeze every manifest/package/bin field. |
| Derived inventory count, ordered names and kind at every position | `product_inventory_and_kinds_are_frozen` compares all 17 complete `(name, ProgramKind)` pairs. Extra/missing entries cannot hide behind `zip`. |
| Derived startup order | `product_startup_order_is_frozen` pins the complete nine-name sequence independently of deployment start and inventory. |

Both deployment tests select the published `product` from `DEPLOYMENTS`, as the
old comparison does; they do not validate an unused `PRODUCT` constant instead.
The inventory and startup tests use that same published selection and the
existing public planner functions.

This maps only `product_deployment_fields_match_pr55` and
`product_inventory_and_startup_match_pr55`. The existing Daemon, selected
Login/UART/RTC namespace/registry/build, and Shell command-port/binding tests
remain unchanged. Provider-local green tests do not replace those product
comparisons. Broader legacy suites remain provisional for migration and
retirement until their assertions and real import closures are mapped.

## Validation

Run the new and retained host suites together:

```sh
cargo test --locked --offline -p vanllia-composition -p vanllia-image --tests --target x86_64-unknown-linux-gnu
```

On Linux x86_64 with the repository's `nightly-2026-09-04`, all five new tests and
all five retained equivalence tests pass. The image adapter's unit target has
zero cases. No product, Kernel, firmware or guest image is built by this check.

An isolated minimal workspace ran 28 negative variants, one at a time. Each
variant produced the intended new test failure, without a compilation failure.
It restored a positive five-test run afterwards and verified that the candidate
source files were untouched.

| Contract | Negative variants |
|---|---|
| Deployment fields (10) | Missing product name; changed entry; reordered, missing or duplicated start; nonempty guarded; reordered or shortened resources; nonempty sources; duplicated published product. |
| Targets/bootstrap (8) | Changed target name/member; reordered scene members or targets; missing/extra target; reordered or shortened bootstrap. |
| Coordinates (3) | Reordered, missing or duplicated coordinate. |
| Inventory (4) | Changed declaration input order; actual System loader kind changed to User; valid missing entry image; valid extra resource plus build coordinate. |
| Startup (3) | Missing or extra actual start item; swapped returned startup positions. |

Twenty-seven variants modify declaration inputs in the isolated workspace. The
startup-order variant perturbs the returned vector immediately before its
comparison; it proves checker sensitivity, not a mutation of the planner's
topological algorithm. No expected literal is changed to make a test pass, and
none of the variants is part of the product or committed Rust tests.

The root lock keeps the same 92 package identities, sources, versions and
checksums. Its only semantic delta is `vanllia-build` in the local composition's
dependency list, reflecting the new `[dev-dependencies]` entry. Locked/offline
host metadata and normal/development dependency graphs resolve. The composition
normal graph is byte-identical to the base graph. Formatting is checked only
for the two new Rust files using the requested toolchain.

## Remaining edges and integration

This slice removes no sqware dependency. The new test expectations do not depend
on old fixtures, but their actual composition still selects the old normal
`shell-declaration`. The retained equivalence target still compiles the frozen
legacy composition and its `fixturetarget-api` and old `rtc-api` dependencies.
The base also still selects Shell and Cat from the legacy build manifest.

Vanllia #7, head `9d540b3e42f27b7904eafee068b3d094e7608eb5`, owns the independent
Shell/Cat and normal Shell declaration intake. Vanllia #6, head
`ff2c059c7df1545afedb95eb74dce71f40ca3f8d`, is a separate boot/acceptance transport
checkpoint. Neither branch is modified here. When integrating, run these new
contracts with the then-current parity suite at the exact joined head; #7 adds a
sixth Shell/Cat comparison that must also remain live. The old Python directory
and sibling-import organization in #6 still needs its own redesign.

This is host declaration/plan coverage only. No QEMU, runtime, SMP, IPC,
interactive Shell, cleanup-in-a-real-Kernel, whole-repository independence or
archive acceptance is claimed. No PR is merged by these checks.
