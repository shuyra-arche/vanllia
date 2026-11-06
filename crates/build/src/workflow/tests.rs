use super::*;

#[test]
fn unit_names_cannot_shadow_tests_or_nested_clusters() {
    let config: Config =
        serde_json::from_str(include_str!("../../examples/pipe/vanllia.json")).unwrap();
    let mut shadow_test = config.clone();
    shadow_test
        .units
        .insert("pipe-integration".into(), config.units["pipe"].clone());
    assert!(
        validate(&shadow_test).is_err(),
        "a Unit could shadow a declared system scenario"
    );
    let mut shadow_cluster = config.clone();
    shadow_cluster
        .units
        .insert("pipe-unit".into(), config.units["pipe"].clone());
    assert!(
        validate(&shadow_cluster).is_err(),
        "a Unit could shadow nested Cluster membership"
    );
}

fn fixture(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("vanllia-{name}-{}", std::process::id()));
    let state = root.join("state");
    let context = root.join("context");
    let out = context.join("attempts/one");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    (root, state, context, out)
}

#[test]
fn publication_failure_is_not_reported_as_success() {
    let (root, state, context, out) = fixture("publish-failure");
    std::fs::create_dir(context.join("current.json")).unwrap();
    std::fs::write(context.join("current.json/retained"), b"previous data").unwrap();
    let result = complete_attempt(
        &state,
        &context,
        &out,
        &["build".into(), "selected".into()],
        Ok(()),
    );
    assert!(result.is_err());
    let outcome: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("outcome.json")).unwrap()).unwrap();
    assert_eq!(outcome["success"], false);
    assert_eq!(outcome["phase"], "failed");
    assert_eq!(
        std::fs::read(context.join("current.json/retained")).unwrap(),
        b"previous data"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unconfirmed_stop_is_reserved_before_failed_outcome_record() {
    let (root, state, context, out) = fixture("reservation-record-failure");
    std::fs::create_dir(out.join("outcome.json")).unwrap();
    let error = complete_attempt(
        &state,
        &context,
        &out,
        &["run".into(), "selected".into()],
        Err("stop_confirmed=false".into()),
    )
    .unwrap_err();
    assert!(error.contains("stop_confirmed=false"), "{error}");
    assert!(error.contains("outcome record failed"), "{error}");
    assert!(state.join("STOP_UNCONFIRMED").is_file());
    assert!(!context.join("current.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}
