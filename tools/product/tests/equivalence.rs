//! Remaining frozen product UnitFile comparisons.
//!
//! Independent plan, image and Shell catalogue contracts live with composition.
//! The legacy dev fixture remains until these declaration fields are migrated.

#[test]
fn daemon_declaration_fields_match_pr55() {
    let provider = &vanllia_composition::units::daemon::UNIT;
    let legacy = &legacy_composition::units::daemon::PROGRAM;

    let slot = vanllia_composition::PROGRAMS
        .iter()
        .position(|program| program.name == "daemon")
        .expect("daemon registry slot");
    assert!(core::ptr::eq(vanllia_composition::PROGRAMS[slot], provider));

    assert_eq!(provider.name, legacy.name);
    assert_eq!(provider.loader.image, legacy.loader.image);
    assert_eq!(provider.loader.space, legacy.loader.space);
    assert_eq!(provider.loader.args, legacy.loader.args);
    assert_eq!(provider.loader.stack, legacy.loader.stack);
    assert_eq!(
        provider.operator.namespace.len(),
        legacy.operator.namespace.len()
    );
    assert_eq!(provider.identity.aliases, legacy.identity.aliases);
    assert_eq!(provider.identity.grants.len(), legacy.identity.grants.len());
    assert_eq!(provider.control.supervision, legacy.control.supervision);
    assert_eq!(provider.control.after, legacy.control.after);
    assert_eq!(provider.control.ending, legacy.control.ending);
    assert_eq!(provider.control.ready, legacy.control.ready);
    assert!(provider.control.construction == legacy.control.construction);
    assert_eq!(provider.control.retirement, legacy.control.retirement);
    assert_eq!(provider.supply.bindings, legacy.supply.bindings);
}

fn assert_observable_fields(actual: &unit::UnitFile, legacy: &unit::UnitFile) {
    assert_eq!(actual.name, legacy.name);
    assert_eq!(actual.loader.image, legacy.loader.image);
    assert_eq!(actual.loader.space, legacy.loader.space);
    assert_eq!(actual.loader.args, legacy.loader.args);
    assert_eq!(actual.loader.stack, legacy.loader.stack);

    assert_eq!(
        actual.operator.namespace.len(),
        legacy.operator.namespace.len()
    );
    for (actual_ns, legacy_ns) in actual
        .operator
        .namespace
        .iter()
        .zip(legacy.operator.namespace)
    {
        assert_eq!(actual_ns.key.name(), legacy_ns.key.name());
        assert!(actual_ns.key.mark() == legacy_ns.key.mark());
        assert!(actual_ns.placement == legacy_ns.placement);
        assert_eq!(actual_ns.path, legacy_ns.path);
        match (actual_ns.scope, legacy_ns.scope) {
            (unit::operator::Scope::Subtree, unit::operator::Scope::Subtree) => {}
            (
                unit::operator::Scope::Names(actual_names),
                unit::operator::Scope::Names(legacy_names),
            ) => assert_eq!(actual_names, legacy_names),
            _ => panic!("namespace scope differs"),
        }
        assert!(actual_ns.ops == legacy_ns.ops);
        assert!(actual_ns.permit == legacy_ns.permit);
        assert_eq!(actual_ns.source_mark, legacy_ns.source_mark);

        assert_eq!(actual_ns.bindings.len(), legacy_ns.bindings.len());
        for (actual_binding, legacy_binding) in
            actual_ns.bindings.iter().zip(legacy_ns.bindings)
        {
            assert_eq!(actual_binding.name, legacy_binding.name);
            assert!(actual_binding.mark == legacy_binding.mark);
            assert!(actual_binding.view == legacy_binding.view);
            assert!(actual_binding.permit == legacy_binding.permit);
        }
    }

    assert_eq!(actual.identity.aliases, legacy.identity.aliases);
    assert!(actual.identity.grants.is_empty());
    assert!(legacy.identity.grants.is_empty());
    assert_eq!(actual.control.supervision, legacy.control.supervision);
    assert_eq!(actual.control.after, legacy.control.after);
    assert_eq!(actual.control.ending, legacy.control.ending);
    assert_eq!(actual.control.ready, legacy.control.ready);
    assert!(actual.control.construction == legacy.control.construction);
    assert_eq!(actual.control.retirement, legacy.control.retirement);
    assert_eq!(actual.supply.bindings, legacy.supply.bindings);
}

#[test]
fn selected_owner_declarations_keep_registry_slots_and_frozen_fields() {
    let cases: [(&str, &unit::UnitFile, &unit::UnitFile); 3] = [
        (
            "login",
            &vanllia_composition::units::login::PROGRAM,
            &legacy_composition::units::login::PROGRAM,
        ),
        (
            "uart",
            &vanllia_composition::units::uart::UNIT,
            &legacy_composition::units::uart::PROGRAM,
        ),
        (
            "rtc",
            &vanllia_composition::units::rtc::UNIT,
            &legacy_composition::units::rtc::PROGRAM,
        ),
    ];

    let expected_registry = [
        "hub", "daemon", "pipe", "account", "terminal", "login", "shell", "router", "uart", "rtc",
        "system",
    ];
    let registry_names: Vec<&str> = vanllia_composition::PROGRAMS
        .iter()
        .map(|program| program.name)
        .collect();
    assert_eq!(registry_names, expected_registry.to_vec());

    for (name, owner, legacy) in cases {
        let slot = vanllia_composition::PROGRAMS
            .iter()
            .position(|program| program.name == name)
            .expect("selected owner registry slot");
        assert!(core::ptr::eq(
            vanllia_composition::PROGRAMS[slot],
            owner
        ));
        assert_observable_fields(owner, legacy);
    }

    assert_eq!(
        vanllia_composition::PROGRAMS
            .iter()
            .filter(|program| program.name == "login")
            .count(),
        1
    );
    assert_eq!(
        vanllia_composition::PROGRAMS
            .iter()
            .filter(|program| program.name == "uart")
            .count(),
        1
    );
    assert_eq!(
        vanllia_composition::PROGRAMS
            .iter()
            .filter(|program| program.name == "rtc")
            .count(),
        1
    );
}

#[test]
fn shell_program_keeps_registry_slot_and_frozen_source_fields() {
    let owner = &vanllia_composition::units::shell::PROGRAM;
    let legacy = &legacy_composition::units::shell::PROGRAM;

    let slot = vanllia_composition::PROGRAMS
        .iter()
        .position(|program| program.name == "shell")
        .expect("shell registry slot");
    assert!(core::ptr::eq(vanllia_composition::PROGRAMS[slot], owner));
    assert_observable_fields(owner, legacy);
    assert_eq!(legacy.name, "shell");
    assert_eq!(legacy.loader.image, "shell");
}
