mod login_probe;
use std::path::{Path, PathBuf};

#[test]
#[ignore = "executes QEMU and the existing public Login fixture; requires exact artifact and firmware inputs"]
fn login_creates_shell_runs_command_and_reclaims_instances() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let artifact =
        PathBuf::from(std::env::var_os("VANLLIA_TEST_ARTIFACT").expect("VANLLIA_TEST_ARTIFACT"));
    let firmware =
        PathBuf::from(std::env::var_os("VANLLIA_TEST_FIRMWARE").expect("VANLLIA_TEST_FIRMWARE"));
    let hash = std::env::var("VANLLIA_TEST_FIRMWARE_SHA256").expect("VANLLIA_TEST_FIRMWARE_SHA256");
    let bytes = std::env::var("VANLLIA_TEST_FIRMWARE_BYTES")
        .expect("VANLLIA_TEST_FIRMWARE_BYTES")
        .parse()
        .expect("firmware length");
    login_probe::run(root, &artifact, &firmware, &hash, bytes, 1).unwrap();
}
