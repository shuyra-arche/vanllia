//! Host-only equivalence against the PR55 legacy product composition.
//!
//! These checks use declarations only; they do not inspect legacy build paths.

use vanllia_build::{image_inventory, startup_order};

fn product(deployments: &'static [unit::Deployment]) -> &'static unit::Deployment {
    deployments
        .iter()
        .find(|deployment| deployment.name == "product")
        .expect("product deployment")
}

#[test]
fn product_deployment_fields_match_pr55() {
    let ours = product(vanllia_composition::DEPLOYMENTS);
    let legacy = product(legacy_composition::DEPLOYMENTS);

    assert_eq!(ours.name, legacy.name);
    assert_eq!(ours.entry, legacy.entry);
    assert_eq!(ours.start, legacy.start);
    assert_eq!(ours.guarded, legacy.guarded);
    assert_eq!(ours.resources, legacy.resources);
    assert_eq!(ours.sources, legacy.sources);
    assert_eq!(ours.targets.len(), legacy.targets.len());
    for (ours, legacy) in ours.targets.iter().zip(legacy.targets) {
        assert_eq!(ours.name, legacy.name);
        assert_eq!(ours.members, legacy.members);
    }
    assert_eq!(
        format!("{:?}", ours.bootstrap),
        format!("{:?}", legacy.bootstrap)
    );
}

#[test]
fn product_inventory_and_startup_match_pr55() {
    let ours = product(vanllia_composition::DEPLOYMENTS);
    let legacy = product(legacy_composition::DEPLOYMENTS);

    let ours_inventory =
        image_inventory(vanllia_composition::PROGRAMS, ours, vanllia_composition::IMAGE_BUILDS)
            .expect("vanllia product inventory");
    let legacy_inventory =
        image_inventory(legacy_composition::PROGRAMS, legacy, legacy_composition::IMAGE_BUILDS)
            .expect("PR55 product inventory");

    assert_eq!(vanllia_composition::IMAGE_BUILDS.len(), 17);
    assert_eq!(ours_inventory.len(), 17);
    assert_eq!(legacy_inventory.len(), 17);

    let ours_names: Vec<&str> = ours_inventory.iter().map(|(name, _)| *name).collect();
    let legacy_names: Vec<&str> = legacy_inventory.iter().map(|(name, _)| *name).collect();
    assert_eq!(ours_names, legacy_names);
    for ((_, ours_kind), (_, legacy_kind)) in ours_inventory.iter().zip(&legacy_inventory) {
        assert_eq!(
            format!("{ours_kind:?}"),
            format!("{legacy_kind:?}"),
            "program kind differs"
        );
    }

    assert_eq!(
        startup_order(vanllia_composition::PROGRAMS, ours).expect("vanllia startup order"),
        startup_order(legacy_composition::PROGRAMS, legacy).expect("PR55 startup order")
    );
}
