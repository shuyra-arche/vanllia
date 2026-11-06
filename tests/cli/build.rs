use initrd::ledger::bundle::Archive;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use vanllia::{
    Selection,
    build::{self, Options},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "vanllia-real-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("src")).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn product() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        r#"
[workspace]
[package]
name = "dynamic-fixture"
version = "0.1.0"
edition = "2024"
[profile.dev]
panic = "abort"
[package.metadata.vanllia]
runtime_resources = ["machine/inventory"]
[package.metadata.vanllia.resources]
settings = "settings.txt"
[[bin]]
name = "system-bin"
path = "src/main.rs"
[[bin]]
name = "worker-bin"
path = "src/main.rs"
[[bin]]
name = "unused-bin"
path = "src/main.rs"
[[bin]]
name = "kernel-bin"
path = "src/main.rs"
"#,
    );
    fixture.write(
        "src/main.rs",
        r#"#![no_std]
#![no_main]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! { loop { core::hint::spin_loop(); } }
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { loop {} }
"#,
    );
    fixture.write(
        "vanllia.toml",
        r#"schema = 1
units = ["worker-a", "worker-b"]
[boot]
system = "system"
[boot.kernel]
image = "kernel-bin"
[platform]
target = "riscv64gc-unknown-none-elf"
"#,
    );
    fixture.write(
        "system/unit.toml",
        "schema=1\nid='system'\n[loader]\nimage='system-bin'\nspace='supervisor'\n",
    );
    fixture.write("a/unit.toml", "schema=1\nid='worker-a'\n[loader]\nimage='worker-bin'\n[control]\nrequires=['worker-b']\n[supply]\nbindings=[{source='system/initrd',resource='settings',slot=10,permission=1},{source='system/initrd',resource='machine/inventory',slot=11,permission=1}]\n");
    fixture.write(
        "b/unit.toml",
        "schema=1\nid='worker-b'\n[loader]\nimage='worker-bin'\n[control]\nafter=['worker-a']\n",
    );
    fixture.write("settings.txt", "host prepared resource");
    fixture
}

fn hash(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

#[test]
fn real_cargo_build_packages_owned_records_and_shared_images_without_a_deployment_file() {
    let fixture = product();
    let receipt =
        build::build(&fixture.0, &Selection::Default, &Options { offline: true }).unwrap();
    assert!(!fixture.0.join("Cargo.lock").exists());
    assert_eq!(receipt.images.len(), 2, "two Units share one Cargo image");
    assert!(!receipt.images.contains_key("unused-bin"));
    let path = receipt.directory.join(&receipt.initrd.path);
    assert_eq!(hash(&path), receipt.initrd.sha256);
    let bytes = std::fs::read(path).unwrap();
    let archive = Archive::parse(&bytes).unwrap();
    assert_eq!(archive.system().id, "system");
    assert_eq!(archive.image().name, "system-bin");
    assert_eq!(archive.units().len(), 3);
    assert!(
        archive.find("machine/inventory").is_none(),
        "System generates runtime inventory"
    );
    for entry in archive.units() {
        let unit = unit_contract::record::decode(entry.record).unwrap();
        assert_eq!(entry.id, unit.id);
        assert!(archive.find(unit.loader.image).is_some());
    }
    assert_eq!(
        archive.find("settings").unwrap().bytes,
        b"host prepared resource"
    );
    assert!(
        archive.find("kernel-bin").is_none(),
        "kernel is a host boot artifact"
    );
    let kernel = receipt.kernel.unwrap();
    assert_eq!(hash(&receipt.directory.join(kernel.path)), kernel.sha256);
    assert!(!receipt.directory.join("deployment.toml").exists());
    assert!(receipt.directory.join("receipt.json").is_file());
    assert!(
        receipt
            .locks
            .values()
            .all(|lock| receipt.directory.join(&lock.path).is_file())
    );
    let loaded = vanllia::runner::load(&receipt.directory).unwrap();
    assert_eq!(loaded.initrd.sha256, receipt.initrd.sha256);
    std::fs::write(
        receipt.directory.join(&receipt.initrd.path),
        b"corrupted initrd",
    )
    .unwrap();
    assert!(vanllia::runner::load(&receipt.directory).is_err());
}

#[test]
fn failed_rebuild_keeps_the_previously_published_artifacts_unchanged() {
    let fixture = product();
    let receipt =
        build::build(&fixture.0, &Selection::Default, &Options { offline: true }).unwrap();
    let path = receipt.directory.join(&receipt.initrd.path);
    let declaration = std::fs::read_to_string(fixture.0.join("a/unit.toml")).unwrap();
    fixture.write(
        "a/unit.toml",
        &declaration.replace("machine/inventory", "missing-resource"),
    );
    assert!(
        matches!(build::build(&fixture.0, &Selection::Default, &Options { offline: true }), Err(vanllia::Error::UnknownResource { resource }) if resource == "missing-resource")
    );
    assert_eq!(hash(&path), receipt.initrd.sha256);
    fixture.write("a/unit.toml", &declaration);
    fixture.write("src/main.rs", "this is not Rust");
    assert!(build::build(&fixture.0, &Selection::Default, &Options { offline: true }).is_err());
    assert_eq!(hash(&path), receipt.initrd.sha256);
    assert_eq!(
        std::fs::read_dir(fixture.0.join(".vanllia/artifacts"))
            .unwrap()
            .count(),
        1
    );
}
