//! Selected product declarations, by observable responsibility.
//! Frozen expectations: sqware 94e114614bec53424ac55e640fc80cf19a3e7107.
//! No legacy crate/source is loaded. Shell resource bindings live in shell_catalogue.rs.

use env::{Access, Mark, ProgramKind};
use system_api::operator::{Ops, Permit};
use unit::{Bootstrap, Dependency, Ending, Ready, UnitFile};
use vanllia_composition::{PROGRAMS, units};

const SELECTED: &[(&str, &UnitFile)] = &[
    ("daemon", &units::daemon::UNIT),
    ("login", &units::login::PROGRAM),
    ("uart", &units::uart::UNIT),
    ("rtc", &units::rtc::UNIT),
    ("shell", &units::shell::PROGRAM),
];

#[test]
fn registry_keeps_product_order_and_selected_provider_ownership() {
    let expected = [
        "hub", "daemon", "pipe", "account", "terminal", "login", "shell", "router", "uart", "rtc",
        "system",
    ];
    let names: Vec<_> = PROGRAMS.iter().map(|program| program.name).collect();
    assert_eq!(names, expected);
    for &(name, owner) in SELECTED {
        let slots: Vec<_> = PROGRAMS
            .iter()
            .filter(|program| program.name == name)
            .collect();
        assert_eq!(slots.len(), 1, "{name} registry membership");
        assert!(core::ptr::eq(*slots[0], owner), "{name} provider ownership");
    }
}

#[test]
fn selected_units_load_named_user_images_without_arguments_or_identity_grants() {
    for &(name, program) in SELECTED {
        assert_eq!(program.name, name);
        assert_eq!(program.loader.image, name, "{name} image");
        assert_eq!(program.loader.space, ProgramKind::User, "{name} space");
        assert!(program.loader.args.is_empty(), "{name} arguments");
        assert_eq!(program.loader.stack, 0, "{name} stack");
        assert!(program.identity.aliases.is_empty(), "{name} aliases");
        assert!(program.identity.grants.is_empty(), "{name} identity grants");
    }
}

#[test]
fn startup_dependencies_readiness_and_termination_keep_product_policy() {
    use Dependency::{Bootstrap as Boot, Target, Unit};
    let cases: &[(
        &UnitFile,
        Option<&[Dependency]>,
        Option<Ending>,
        Ready,
        Option<&str>,
    )] = &[
        (
            &units::daemon::UNIT,
            Some(&[Boot(Bootstrap::Operator), Target("supervised")]),
            Some(Ending::Resident),
            Ready::Signal(Mark::of("ready")),
            Some("supervised"),
        ),
        (
            &units::login::PROGRAM,
            Some(&[
                Boot(Bootstrap::Operator),
                Unit("terminal"),
                Unit("account"),
                Target("scene"),
            ]),
            Some(Ending::Told),
            Ready::OnSpawn,
            None,
        ),
        (
            &units::uart::UNIT,
            Some(&[Boot(Bootstrap::Operator), Unit("hub"), Unit("router")]),
            Some(Ending::Resident),
            Ready::Signal(Mark::of("ready")),
            None,
        ),
        (
            &units::rtc::UNIT,
            Some(&[Boot(Bootstrap::Operator), Unit("hub"), Unit("router")]),
            Some(Ending::Resident),
            Ready::Signal(Mark::of("ready")),
            None,
        ),
        (&units::shell::PROGRAM, None, None, Ready::OnSpawn, None),
    ];
    for &(program, after, ending, ready, supervision) in cases {
        assert_eq!(
            program.control.after, after,
            "{} dependency order",
            program.name
        );
        assert_eq!(
            program.control.ending, ending,
            "{} termination",
            program.name
        );
        assert_eq!(program.control.ready, ready, "{} readiness", program.name);
        assert_eq!(
            program.control.supervision, supervision,
            "{} supervision",
            program.name
        );
    }
}

#[test]
fn only_shell_constructs_images_and_selected_units_have_no_retirement_actions() {
    use unit::construction::{Identity, Owner, Selector};
    for &(name, program) in SELECTED {
        assert!(program.control.retirement.is_empty(), "{name} retirement");
        if name != "shell" {
            assert!(
                program.control.construction.is_empty(),
                "{name} construction"
            );
            assert!(program.supply.bindings.is_empty(), "{name} supplies");
        }
    }
    let policies = units::shell::PROGRAM.control.construction;
    assert_eq!(policies.len(), 1);
    let policy = policies[0];
    assert!(policy.selector == Selector::Image);
    assert!(policy.owner == Owner::Caller);
    assert!(policy.identity == Identity::Caller);
    assert_eq!((policy.arguments.min, policy.arguments.max), (0, 64));
    assert_eq!((policy.stack.min, policy.stack.max), (0, 1 << 20));
    assert_eq!(policy.max_image, 16 * 1024 * 1024);
}

#[test]
fn driver_publication_keeps_shared_service_scope_and_rtc_entry_binding() {
    use unit::operator::{Placement, Scope};
    for program in [
        &units::daemon::UNIT,
        &units::login::PROGRAM,
        &units::shell::PROGRAM,
    ] {
        assert!(
            program.operator.namespace.is_empty(),
            "{} namespaces",
            program.name
        );
    }
    let expected_ops = Ops::PART
        .union(Ops::UNPART)
        .union(Ops::BIND)
        .union(Ops::UNBIND)
        .union(Ops::FIND);
    for (name, program, bindings) in [
        ("uart", &units::uart::UNIT, 0),
        ("rtc", &units::rtc::UNIT, 1),
    ] {
        assert_eq!(
            program.operator.namespace.len(),
            1,
            "{name} namespace count"
        );
        let namespace = program.operator.namespace[0];
        assert_eq!(namespace.key.name(), "service");
        assert!(namespace.key.mark() == system_api::operator::NamespaceKey::of("service").mark());
        assert!(namespace.placement == Placement::Shared);
        assert_eq!(namespace.path, name);
        assert!(matches!(namespace.scope, Scope::Subtree));
        assert!(namespace.ops == expected_ops);
        assert!(namespace.permit == Some(Permit::Public));
        assert_eq!(namespace.source_mark, None);
        assert_eq!(namespace.bindings.len(), bindings, "{name} binding count");
    }
    let binding = units::rtc::UNIT.operator.namespace[0].bindings[0];
    assert_eq!(binding.name, "entry");
    assert!(binding.mark == Mark::of("entry"));
    assert!(binding.view == Access::STORE);
    assert!(binding.permit == Permit::Public);
}
