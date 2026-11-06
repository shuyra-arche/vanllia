//! Frozen host contract assertions for the product deployment.
//!
//! Expectations are inline literals independently sourced from the immutable
//! sqware product declaration at
//! `94e114614bec53424ac55e640fc80cf19a3e7107` and the planning API observation
//! at `17fd7ddbd18b3aef3a8d348cd32d8642e81b3c7f`. The expectation provenance
//! is both the old product declaration and the current planning API.

use vanllia_composition::DEPLOYMENTS;

fn product() -> &'static unit::Deployment {
    DEPLOYMENTS
        .iter()
        .find(|deployment| deployment.name == "product")
        .expect("product deployment")
}

#[test]
fn product_deployment_fields_are_frozen() {
    assert_eq!(
        DEPLOYMENTS
            .iter()
            .filter(|deployment| deployment.name == "product")
            .count(),
        1,
        "exactly one product deployment must be published"
    );

    let deployment = product();

    assert_eq!(deployment.name, "product");
    assert_eq!(deployment.entry, "system");
    assert_eq!(
        deployment.start.to_vec(),
        vec![
            "hub", "pipe", "account", "terminal", "login", "router", "uart", "rtc", "daemon",
        ]
    );
    assert!(deployment.guarded.is_empty(), "guarded must be empty");
    assert_eq!(
        deployment.resources.to_vec(),
        vec!["shell", "cat", "emit", "upper", "fail", "spin", "workers"]
    );
    assert!(deployment.sources.is_empty(), "sources must be empty");
}

#[test]
fn product_targets_and_bootstrap_are_frozen() {
    let deployment = product();

    assert_eq!(deployment.targets.len(), 2);
    assert_eq!(deployment.targets[0].name, "scene");
    assert_eq!(
        deployment.targets[0].members.to_vec(),
        vec![
            "hub", "pipe", "account", "terminal", "router", "uart", "rtc", "daemon",
        ]
    );
    assert_eq!(deployment.targets[1].name, "supervised");
    assert_eq!(deployment.targets[1].members.to_vec(), vec!["rtc"]);

    assert_eq!(deployment.bootstrap.len(), 2);
    assert!(
        matches!(&deployment.bootstrap[0], unit::Bootstrap::Operator),
        "bootstrap[0] must be Operator"
    );
    assert!(
        matches!(&deployment.bootstrap[1], unit::Bootstrap::Identity),
        "bootstrap[1] must be Identity"
    );
}
