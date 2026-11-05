//! Shell command ports, supply bindings and product image resources.
//! Frozen command data: sqware 94e114614bec53424ac55e640fc80cf19a3e7107.
//! No legacy crate or source file is loaded by these tests.

use env::{Mark, Permission};
use pipe_api::Direction::{Read, Write};
use system_api::supply::Key;
use unit::Binding;
use vanllia_composition::{DEPLOYMENTS, IMAGE_BUILDS, units::shell};

const fn expected_binding(resource: &'static str, slot: &'static str) -> Binding {
    Binding {
        source: Key::of("system/initrd"),
        resource: Key::of(resource),
        slot: Mark::of(slot),
        permission: Permission::FETCH,
    }
}

const EXPECTED_BINDINGS: &[Binding] = &[
    expected_binding("cat", "shell-cat"),
    expected_binding("emit", "shell-emit"),
    expected_binding("upper", "shell-upper"),
    expected_binding("fail", "shell-fail"),
    expected_binding("spin", "shell-spin"),
    expected_binding("workers", "shell-workers"),
];

#[test]
fn shell_catalogue_keeps_ordered_names_ports_and_directions() {
    let expected: &[(&str, &[(&str, pipe_api::Direction)])] = &[
        ("cat", &[("source", Read), ("copy", Write)]),
        ("emit", &[("records", Write)]),
        ("upper", &[("source", Read), ("result", Write)]),
        ("fail", &[]),
        ("spin", &[]),
        ("workers", &[("ticks", Write)]),
    ];
    assert_eq!(shell::COMMANDS.len(), expected.len());
    for (command, (name, ports)) in shell::COMMANDS.iter().zip(expected) {
        assert_eq!(command.name, *name);
        assert_eq!(command.ports, *ports, "{}", name);
    }
}

#[test]
fn command_and_program_supply_bindings_keep_frozen_keys_slots_permissions() {
    assert_eq!(shell::COMMANDS.len(), EXPECTED_BINDINGS.len());
    for (command, expected) in shell::COMMANDS.iter().zip(EXPECTED_BINDINGS) {
        assert_eq!(&command.binding, expected, "{}", command.name);
    }
    assert_eq!(shell::BINDINGS, EXPECTED_BINDINGS);
    assert_eq!(shell::PROGRAM.supply.bindings, EXPECTED_BINDINGS);
}

#[test]
fn catalogue_commands_are_selected_product_resources_with_expected_bins() {
    let product = DEPLOYMENTS
        .iter()
        .find(|deployment| deployment.name == "product")
        .expect("published product");
    assert_eq!(shell::COMMANDS.len(), 6);
    for command in shell::COMMANDS {
        assert!(
            product.resources.contains(&command.name),
            "{} is not a product resource",
            command.name
        );
        let builds: Vec<_> = IMAGE_BUILDS
            .iter()
            .filter(|build| build.image == command.name)
            .collect();
        assert_eq!(builds.len(), 1, "{} must have one image", command.name);
        assert_eq!(
            builds[0].bin,
            if command.name == "cat" {
                "prog-cat"
            } else {
                command.name
            }
        );
    }
}
