use std::collections::BTreeMap;
use std::path::PathBuf;
use vanllia::{
    Catalog, Error, Selection, catalog,
    config::{Boot, Project},
    image::Image,
    reason::Reason,
    unit::{Control, Declaration, Loader, Space, Unit},
};

fn declaration(id: &str, requires: &[&str], after: &[&str]) -> Declaration {
    Declaration {
        path: PathBuf::from(format!("/{id}/unit.toml")),
        unit: Unit {
            schema: 1,
            id: id.into(),
            loader: Loader {
                image: id.into(),
                space: if id == "system" {
                    Space::Supervisor
                } else {
                    Space::User
                },
                args: vec![],
                stack: 0,
            },
            control: Control {
                requires: requires.iter().map(|id| (*id).into()).collect(),
                after: after.iter().map(|id| (*id).into()).collect(),
                ..Control::default()
            },
            ..Unit::default()
        },
    }
}

fn index(mut units: Vec<Declaration>, roots: &[&str]) -> Result<Catalog, Error> {
    units.push(declaration("system", &[], &[]));
    let images = units
        .iter()
        .map(|declaration| Image {
            name: declaration.unit.loader.image.clone(),
            manifest: declaration.path.with_file_name("Cargo.toml"),
            workspace: declaration.path.parent().unwrap().to_path_buf(),
            package: declaration.unit.id.clone(),
            package_id: declaration.unit.id.clone(),
        })
        .collect();
    catalog::index(
        Project {
            schema: 1,
            units: roots.iter().map(|id| (*id).into()).collect(),
            boot: Boot {
                system: "system".into(),
                kernel: None,
                firmware: None,
            },
            platform: None,
            runner: None,
            sources: BTreeMap::new(),
            overrides: BTreeMap::new(),
        },
        units,
        images,
    )
}

#[test]
fn requires_cycles_are_a_finite_selection_without_waiting_edges() {
    let catalog = index(
        vec![declaration("a", &["b"], &[]), declaration("b", &["a"], &[])],
        &["a"],
    )
    .unwrap();
    let plan = vanllia::plan(&catalog, &Selection::Default).unwrap();
    assert_eq!(plan.order, ["system", "a", "b"]);
    assert_eq!(plan.reasons["b"], [Reason::RequiredBy("a".into())]);
    assert!(plan.reasons["a"].contains(&Reason::RequiredBy("b".into())));
}

#[test]
fn after_does_not_select_an_available_unit() {
    let catalog = index(
        vec![declaration("a", &[], &["b"]), declaration("b", &[], &[])],
        &["a"],
    )
    .unwrap();
    assert_eq!(
        vanllia::plan(&catalog, &Selection::Default).unwrap().order,
        ["system", "a"]
    );
}

#[test]
fn selected_after_edges_order_dependencies_and_reject_cycles() {
    let catalog = index(
        vec![declaration("a", &["b"], &["b"]), declaration("b", &[], &[])],
        &["a"],
    )
    .unwrap();
    assert_eq!(
        vanllia::plan(&catalog, &Selection::Default).unwrap().order,
        ["system", "b", "a"]
    );
    let cyclic = index(
        vec![
            declaration("a", &["b"], &["b"]),
            declaration("b", &[], &["a"]),
        ],
        &["a"],
    )
    .unwrap();
    assert!(matches!(
        vanllia::plan(&cyclic, &Selection::Default),
        Err(Error::Cycle(_))
    ));
}

#[test]
fn cycles_outside_the_selection_do_not_block_a_deployment() {
    let catalog = index(
        vec![
            declaration("a", &[], &["b"]),
            declaration("b", &[], &["a"]),
            declaration("c", &[], &[]),
        ],
        &["c"],
    )
    .unwrap();
    assert_eq!(
        vanllia::plan(&catalog, &Selection::Default).unwrap().order,
        ["system", "c"]
    );
}

#[test]
fn explicit_selection_replaces_product_defaults_and_keeps_system() {
    let catalog = index(
        vec![
            declaration("desktop", &[], &[]),
            declaration("probe", &[], &[]),
        ],
        &["desktop"],
    )
    .unwrap();
    assert_eq!(
        vanllia::plan(&catalog, &Selection::Explicit(vec!["probe".into()]))
            .unwrap()
            .order,
        ["system", "probe"]
    );
    assert!(matches!(
        vanllia::plan(&catalog, &Selection::Explicit(vec![])),
        Err(Error::EmptySelection)
    ));
}

#[test]
fn all_references_must_exist_even_when_the_target_would_not_be_selected() {
    assert!(
        matches!(index(vec![declaration("a", &[], &["typo"])], &["a"]), Err(Error::UnknownUnit { id, .. }) if id == "typo")
    );
    assert!(
        matches!(index(vec![declaration("a", &["missing"], &[])], &["a"]), Err(Error::UnknownUnit { id, .. }) if id == "missing")
    );
}

#[test]
fn conflicting_bin_names_are_reported_with_both_sources() {
    let mut second = declaration("b", &[], &[]);
    second.unit.loader.image = "a".into();
    let error = index(vec![declaration("a", &[], &[]), second], &["a"]).unwrap_err();
    assert!(
        matches!(error, Error::Duplicate { kind: "Cargo bin", first, second, .. } if first != second)
    );
}

#[test]
fn duplicate_unit_ids_do_not_overwrite_a_declaration() {
    assert!(matches!(
        index(
            vec![declaration("a", &[], &[]), declaration("a", &[], &[])],
            &["a"]
        ),
        Err(Error::Duplicate {
            kind: "Unit ID",
            ..
        })
    ));
}

#[test]
fn system_cannot_wait_for_units_before_reading_the_initrd() {
    let mut system = declaration("system", &["a"], &[]);
    system.unit.id = "bootstrap".into();
    // Use this declaration as the actual entry rather than adding a second System.
    let catalog = index(vec![declaration("a", &[], &[])], &["a"]).unwrap();
    let units = catalog
        .units()
        .values()
        .filter(|unit| unit.unit.id != "system")
        .cloned()
        .chain([system])
        .collect();
    let images = catalog.images().values().cloned().collect();
    let mut project = catalog.project().clone();
    project.boot.system = "bootstrap".into();
    assert!(matches!(
        catalog::index(project, units, images),
        Err(Error::Declaration { .. })
    ));
    // The untouched original remains usable after a rejected independent plan.
    assert!(vanllia::plan(&catalog, &Selection::Default).is_ok());
}

#[test]
fn root_order_does_not_change_the_planned_set_or_startup_order() {
    let catalog = index(
        vec![
            declaration("a", &["c"], &[]),
            declaration("b", &["c"], &["c"]),
            declaration("c", &[], &[]),
        ],
        &["a", "b"],
    )
    .unwrap();
    let left = vanllia::plan(&catalog, &Selection::Explicit(vec!["a".into(), "b".into()])).unwrap();
    let right =
        vanllia::plan(&catalog, &Selection::Explicit(vec!["b".into(), "a".into()])).unwrap();
    assert_eq!(left, right);
}
