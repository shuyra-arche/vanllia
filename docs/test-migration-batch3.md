# Product declaration contracts without the legacy dev fixture

Base: merged main `7e284f600b919d9e19fcdae6b96352e1c8a4ef79`.
The retired `tools/product/tests/equivalence.rs` is Git blob
`ec401bccd3036d70e1c05e765b11fe2f05499278`. Its three cases read five frozen
UnitFiles; this batch replaces those comparisons, without moving old test
directories or importing old source through another path.

`composition/tests/unit_contracts.rs` owns five cross-provider product contracts,
organized by registration, launch/identity, lifecycle, construction/resources
and driver publication. Expectations are test-only literals and public enum
variants. Existing `shell_catalogue.rs` owns the six complete Shell bindings;
that oracle is reused through the test gate, not duplicated in another array.
No production declaration, provider pin, image coordinate, guest scene or
implementation algorithm changes in this migration.

## Retained assertions

Frozen sqware source: `94e114614bec53424ac55e640fc80cf19a3e7107`.

| Frozen declaration | Source path | Blob |
|---|---|---|
| Daemon | `crates/composition/src/units/daemon.rs` | `07595a94c476476f4ab9a91ebe5193d12cbbce8f` |
| Login | `crates/composition/src/units/login.rs` | `8719f76610082b2e8c5166a64d499934f0265a26` |
| UART | `crates/composition/src/units/uart.rs` | `cd0f0d00258f5146406a718b84b7e71776a64f94` |
| RTC | `crates/composition/src/units/rtc.rs` | `421c63898674832662d18347e37cd9c1a5b2dd90` |
| Shell | `programs/src/user/shell/declaration/src/program.rs` | `49d867f35569f9ed1cdb31dc5eddf45ba5797574` |

| Retired comparison/field | Independent successor |
|---|---|
| Daemon registry pointer; selected-owner ordered eleven-name registry, Login/UART/RTC slots and uniqueness; Shell slot/pointer | `registry_keeps_product_order_and_selected_provider_ownership`: same order; all five names occur exactly once and point to the selected provider item. |
| Five declaration names and loader image/space/args/stack; aliases and grants | `selected_units_load_named_user_images_without_arguments_or_identity_grants`: literal names/images, User space, empty arguments, zero stack, no aliases/grants. Frozen Daemon's namespace/grant lengths were zero; new checks require emptiness. |
| Supervision, ordered optional dependencies, ending, readiness for all five | `startup_dependencies_readiness_and_termination_keep_product_policy`: distinguishes absent and empty dependencies, explicit `scene`/`supervised` strings, Bootstrap/Unit/Target variants, Resident/Told and OnSpawn/ready signal. |
| Construction, retirement and supply slices | `only_shell_constructs_images_and_selected_units_have_no_retirement_actions`: four empty construction/supply slices and all five empty retirement slices. Shell has exactly one image policy, caller owner/identity, argument bounds 0–64, stack bounds 0–1 MiB and 16 MiB image limit. Existing `command_and_program_supply_bindings_keep_frozen_keys_slots_permissions` retains ordered complete Shell supplies. |
| Namespace lengths, key name/mark, placement/path/scope/ops/permit/source mark, binding count and ordered binding fields | `driver_publication_keeps_shared_service_scope_and_rtc_entry_binding`: three empty namespace slices; UART/RTC each have one shared service subtree with literal path, complete five-operation mask, public permit, no source mark. UART has no binding; RTC has one `entry`/entry-mark/STORE/public binding. No Names scope exists in these frozen declarations; the Subtree check rejects a switch to Names. |
| Old Shell fixture self-checks `name == "shell"`, `loader.image == "shell"` | The launch contract applies these literals directly to the selected Shell declaration; no legacy oracle remains. |

Typed keys/marks use the existing public encoders with literal names, rather
than the producer's constants. This retains the old comparison's API identity;
it does not independently test the encoding/hash algorithms. Shell's old
`Construction::image()` was compared through a shared SDK constructor. The new
case fixes every public field separately, including the selected Unit SDK's
`90396f0fda376e0693fb1b9553aa21864d241057` constructor values.

## Dependency removal

The old three cases and new five cases first passed together: **22** Rust
tests, versus the original **17**. Only then were the old file and
`tools/product`'s `legacy-composition` dev edge removed. The final gate has
**19** Rust tests, including the unchanged 14 previous composition cases.

Cargo regenerated/pruned the lock offline. Its 95 package identities become
86, with no additions, upgrades, source changes or checksum changes among
survivors. Four removed identities came from frozen sqware: composition,
shell-declaration, rtc-api and fixturetarget-api. The other five were only
reachable through that fixture: superseded Driver/Session/Terminal declarations
and PCI/IOMMU declarations. Cargo shortens now-unambiguous dependency spellings;
resolved edges between surviving packages remain identical. The root production
normal graph is byte-identical, and the all-kind locked graph has **zero sqware
packages**. Provider workspace locks and pins are unchanged.

## Validation and limits

Linux x86_64; repository `nightly-2026-09-04`, Python 3.12.14, Nushell 0.107.0:

```sh
cargo test --locked --offline -p vanllia-composition -p vanllia-image \
  --tests --target x86_64-unknown-linux-gnu
python3 -B -m unittest discover -s tests -t .
nu scripts/build.nu release
```

The first isolated host run passes all 19 Rust cases without any initialized
provider or legacy submodule. Its separate Cargo source cache excludes both
sqware Git databases and checkouts; registry downloads and other pinned Git
sources are available offline. This is old-source independence for the checked
closures, not an empty-cache or hermetic-build claim.

Forty-one private declaration/registry-input variants fail their intended
assertions, with zero compile failures, followed by a five-case passing restored
control. They vary names, loader/identity fields, dependency order/lifecycle,
each Shell construction bound/selector/owner/identity, extra retirement/supply,
namespace and RTC binding fields, registry order/duplicate membership and an
identical-value registry copy with the wrong pointer. Each run checks original
bytes before mutation and complete restored bytes afterwards. Variants use
local declaration adapters in the isolated composition; immutable provider
objects, the real candidate and shared Cargo cache are never written. These are
assertion/input sensitivity checks, not provider or SDK algorithm mutation
coverage.

After those host checks, the same isolated checkout initialized 16 selected
provider/nested checkouts at their exact gitlinks, leaving `legacy/` absent.
The actual `nu scripts/build.nu release` entry passed offline using the
sqware-free source cache; its product and Kernel target directories were fresh.
It produced one JSON record and 17 packed programs. The artifacts are:

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Product initrd | 2,920,448 | `52aedd8f655c62a95213602e3fc217e6dcdbd1a99cb1fee7abd266a7be1160ce` |
| Kernel ELF | 1,887,000 | `27e2cdb531a8fcbfc6c5a19bf18bcfc68aed50ef63d69f128cc80fbc9f1bd7d8` |

These hashes differ from the earlier checkout's build. Checkout and Cargo
source-cache paths changed; no bit-for-bit reproducibility claim is made. No
production source, provider pin or remaining resolved dependency edge changed.
The selected System provider's actual ELF parser accepts all 17 packed images;
the SDK manifest and bootstrap capsule parsers also pass, with the expected
ordered inventory, User/Supervisor kinds and 16 MiB image limit. Static ELF
inspection places the Kernel entry at `0x80200000`. These are host/static checks.

The same result includes the user-requested QEMU default firmware correction;
see [firmware selection](firmware.md). Five additional Python host cases cover
default/explicit argument selection, runner directory changes, custom-file
failure and synthetic startup errors. The original 305-case discovery and
assertion fingerprints remain unchanged. In the two original firmware-negative
setups, an invalid explicit path replaces the now-valid unset variable; that
behavior change is intentional. No emulator is invoked by these tests.

Raw overlap, graph/lock, isolated, negative, Python and build evidence is kept
outside the repository under `boot-review/test-migration-batch3/` in the execution
workspace. It is a recorded local result, not a release gate or approval.

The physical `legacy/sqware` gitlink, `.gitmodules` entry, workspace exclusion,
historical provenance, source inventory and 35 disabled guest-scene gates
remain. `generate-composition-inputs.py` still needs the historical top-level
Git objects, and execution-record source checking inherits that requirement.
Other old host suites still need their own ownership/assertion mapping. The
`sqware-diagnose.jsonl` and related trace names are existing protocol/transport
names. Nothing here authorizes removing the checkout, archiving the old repo,
publishing, runtime acceptance or SMP.
