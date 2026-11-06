//! Namespace policy contracts owned by product composition.
//!
//! The first three cases preserve the frozen publication-admission assertions
//! from sqware 94e114614bec53424ac55e640fc80cf19a3e7107. They consume the
//! selected public Unit/System APIs, with no legacy source or fixture imports.

extern crate alloc;

use system_api::{
    identity::{CoalitionId, Selector},
    operator::{Ops, Path, Permit, Scope as GrantedScope},
};
use unit::operator::{Namespace, Placement, Scope};

fn shared_namespace(scope: Scope, permit: Option<Permit>) -> Namespace {
    Namespace {
        key: system_api::namespace::SERVICE,
        placement: Placement::Shared,
        path: "arbitrary/layout",
        scope,
        ops: Ops::BIND.union(Ops::FIND),
        permit,
        source_mark: None,
        bindings: &[],
    }
}

#[test]
fn composition_controls_layout_and_bounded_names_without_service_classes() {
    let policy = shared_namespace(Scope::Names(&["first", "second"]), None);
    assert!(policy.valid());
    assert_eq!(policy.path, "arbitrary/layout");
    let scope = GrantedScope::Names(alloc::vec![
        Path::new("first").to_path_buf(),
        Path::new("second").to_path_buf(),
    ]);
    assert!(scope.allows("first"));
    assert!(!scope.allows("unlisted"));
    assert!(!scope.allows("../escape"));
    let mut invalid = policy;
    invalid.path = "../escape";
    assert!(!invalid.valid());
}

#[test]
fn finite_grants_can_narrow_but_cannot_escape_the_installed_scope() {
    let parent = GrantedScope::Names(alloc::vec![
        Path::new("first").to_path_buf(),
        Path::new("second").to_path_buf(),
    ]);
    assert!(parent.narrows(&GrantedScope::Names(alloc::vec![
        Path::new("first").to_path_buf(),
    ])));
    assert!(!parent.narrows(&GrantedScope::Names(alloc::vec![
        Path::new("third").to_path_buf(),
    ])));
    assert!(!parent.narrows(&GrantedScope::Subtree));
}

#[test]
fn permission_choice_is_explicit_and_shared_base_requires_no_extra_directory() {
    let member = Permit::Identity(Selector::MemberOf(CoalitionId::new(env::TaskId::new(1), 0)));
    for permit in [
        None,
        Some(Permit::Public),
        Some(Permit::Bound),
        Some(member),
    ] {
        let mut policy = shared_namespace(Scope::Subtree, permit);
        assert!(policy.valid());
        assert_eq!(policy.permit, permit);
        policy.path = "";
        assert!(policy.valid(), "explicit shared base is valid");
    }
    assert!(!shared_namespace(Scope::Names(&[]), None).valid());
}

#[test]
fn registered_product_namespace_policies_satisfy_admission() {
    let mut checked = 0;
    for program in vanllia_composition::PROGRAMS {
        for namespace in program.operator.namespace {
            assert!(
                namespace.valid(),
                "{} declares an invalid {} namespace at {:?}",
                program.name,
                namespace.key.name(),
                namespace.path,
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the published registry must contain namespace policies"
    );
}
