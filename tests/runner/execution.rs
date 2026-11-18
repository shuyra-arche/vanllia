//! Regression for the Host process adapter used by Native QEMU verification.
//! Executed without a VM; target contracts remain distinct.
use std::{fs, path::Path, process::Command};
use vanllia_build::workflow::process;

#[test]
fn preserves_a_nonzero_native_exit_without_approving_it() {
    process::initialize().unwrap();
    let dir = std::env::temp_dir().join(format!("vanllia-test-exit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let status = process::observe(
        Command::new("sh").args(["-c", "exit 17"]),
        &dir,
        5,
    )
    .unwrap();
    assert_eq!(status.code(), Some(17));
    assert!(!status.success());
    let evidence: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("exit.json")).unwrap()).unwrap();
    assert_eq!(evidence["code"], 17);
    assert_eq!(evidence["success"], false);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_timeout_and_preserves_failure_evidence() {
    process::initialize().unwrap();
    let dir = std::env::temp_dir().join(format!("vanllia-test-timeout-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let error = process::observe(
        Command::new("sh").args(["-c", "sleep 30"]),
        &dir,
        1,
    )
    .unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    let evidence: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("result.json")).unwrap()).unwrap();
    assert_eq!(evidence["success"], false);
    assert!(!dir.join("STOP_UNCONFIRMED").exists(), "{error}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rejects_leftover_descendants_after_a_clean_leader_exit() {
    process::initialize().unwrap();
    let dir = std::env::temp_dir().join(format!("vanllia-test-descendant-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let error = process::observe(
        Command::new("sh").args(["-c", "sleep 30 & echo $!"]),
        &dir,
        5,
    )
    .unwrap_err();
    assert!(error.contains("runner left active descendants"), "{error}");
    let child = fs::read_to_string(dir.join("stdout.log")).unwrap();
    assert!(
        !Path::new(&format!("/proc/{}", child.trim())).exists(),
        "{error}"
    );
    fs::remove_dir_all(dir).unwrap();
}
