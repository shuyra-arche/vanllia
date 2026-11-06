//! Frozen host contract assertions for product image coordinates and inventory.
//!
//! Expectations are inline literals independently sourced from the immutable
//! sqware product declaration at
//! `94e114614bec53424ac55e640fc80cf19a3e7107` and the planning API observation
//! at `17fd7ddbd18b3aef3a8d348cd32d8642e81b3c7f`. The expectation provenance
//! is both the old product declaration and the current planning API.

use env::ProgramKind;
use vanllia_build::{image_inventory, startup_order};
use vanllia_composition::{DEPLOYMENTS, IMAGE_BUILDS, PROGRAMS};

fn product() -> &'static unit::Deployment {
    DEPLOYMENTS
        .iter()
        .find(|deployment| deployment.name == "product")
        .expect("product deployment")
}

#[test]
fn image_build_coordinate_order_is_frozen() {
    assert_eq!(IMAGE_BUILDS.len(), 17);

    let images: Vec<&str> = IMAGE_BUILDS.iter().map(|build| build.image).collect();
    assert_eq!(
        images,
        vec![
            "hub", "router", "pipe", "account", "terminal", "system", "login", "uart", "rtc",
            "daemon", "shell", "cat", "emit", "upper", "fail", "spin", "workers",
        ]
    );
}

#[test]
fn product_inventory_and_kinds_are_frozen() {
    let deployment = product();
    let inventory = image_inventory(PROGRAMS, deployment, IMAGE_BUILDS).expect("product inventory");

    let expected: Vec<(&str, ProgramKind)> = vec![
        ("daemon", ProgramKind::User),
        ("rtc", ProgramKind::User),
        ("uart", ProgramKind::User),
        ("router", ProgramKind::User),
        ("login", ProgramKind::User),
        ("terminal", ProgramKind::User),
        ("account", ProgramKind::User),
        ("shell", ProgramKind::User),
        ("cat", ProgramKind::User),
        ("emit", ProgramKind::User),
        ("upper", ProgramKind::User),
        ("fail", ProgramKind::User),
        ("spin", ProgramKind::User),
        ("workers", ProgramKind::User),
        ("pipe", ProgramKind::User),
        ("hub", ProgramKind::User),
        ("system", ProgramKind::Supervisor),
    ];

    assert_eq!(inventory.len(), 17);
    assert_eq!(inventory.len(), expected.len());

    let actual: Vec<(&str, ProgramKind)> = inventory
        .iter()
        .map(|(name, kind)| (*name, *kind))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn product_startup_order_is_frozen() {
    let deployment = product();
    let startup = startup_order(PROGRAMS, deployment).expect("product startup order");

    assert_eq!(
        startup.to_vec(),
        vec![
            "account", "hub", "pipe", "router", "rtc", "daemon", "uart", "terminal", "login",
        ]
    );
}
