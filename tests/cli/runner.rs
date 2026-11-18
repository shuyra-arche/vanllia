use vanllia::config::Runner;

#[test]
fn runner_defaults_to_four_harts_without_a_cpu_override() {
    let runner: Runner = serde_json::from_str("{}").unwrap();
    assert_eq!(runner.cpus, 4);
    assert_eq!(runner.machine, "virt");
}

#[test]
fn runner_preserves_explicit_single_hart_selection() {
    let runner: Runner = serde_json::from_str(r#"{"cpus":1}"#).unwrap();
    assert_eq!(runner.cpus, 1);
}
