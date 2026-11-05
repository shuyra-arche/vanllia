mod probe;
use std::path::{Path, PathBuf};

#[test]
#[ignore = "executes single-core QEMU; requires explicit artifact and hashed firmware inputs"]
fn product_eof_reclaims_all_native_instances() {
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
    probe::run(root, &artifact, &firmware, &hash, bytes).unwrap();
}
