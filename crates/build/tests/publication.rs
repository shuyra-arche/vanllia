use std::path::{Path, PathBuf};

use unit::{
    ControlConfig, Deployment, Identity, LoaderConfig, OperatorConfig, SupplyConfig, UnitFile,
};
use vanllia_build::{ImageBuild, build_with, executables};

static ENTRY: UnitFile = UnitFile {
    name: "entry",
    loader: LoaderConfig::supervisor("entry-image"),
    operator: OperatorConfig::DEFAULT,
    identity: Identity::DEFAULT,
    control: ControlConfig::DEFAULT,
    supply: SupplyConfig::DEFAULT,
};
static GOOD: Deployment = Deployment {
    name: "publication",
    entry: "entry",
    start: &[],
    guarded: &[],
    resources: &[],
    targets: &[],
    bootstrap: &[],
    sources: &[],
};
static BAD: Deployment = Deployment {
    name: "deliberately-too-long-deployment-name-for-manifest",
    entry: "entry",
    start: &[],
    guarded: &[],
    resources: &[],
    targets: &[],
    bootstrap: &[],
    sources: &[],
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/link-workspace")
}
fn build() -> ImageBuild {
    ImageBuild {
        image: "entry-image",
        manifest: "first/Cargo.toml",
        package: "link-first",
        bin: "first",
        features: &[],
        target: "riscv64gc-unknown-none-elf",
        profile: "dev",
        default_features: true,
    }
}

#[test]
fn missing_mapping_cargo_and_pack_failures_preserve_last_complete_image() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("publication-failure");
    let output = work.join("initrd.img");
    build_with(&root(), &work, &output, &[&ENTRY], &GOOD, &[build()]).unwrap();
    let successful = std::fs::read(&output).unwrap();
    assert!(
        build_with(&root(), &work, &output, &[&ENTRY], &GOOD, &[])
            .unwrap_err()
            .contains("未登记镜像")
    );
    assert_eq!(std::fs::read(&output).unwrap(), successful);
    let mut broken = build();
    broken.bin = "not-a-real-bin";
    assert!(build_with(&root(), &work, &output, &[&ENTRY], &GOOD, &[broken]).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), successful);
    let error = build_with(&root(), &work, &output, &[&ENTRY], &BAD, &[build()]).unwrap_err();
    assert!(error.contains("非法部署或镜像清单"), "{error}");
    assert_eq!(std::fs::read(&output).unwrap(), successful);
    assert!(!std::fs::read_dir(&work).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".vanllia-")
    }));
}

#[test]
fn later_cargo_rebuild_does_not_mutate_returned_snapshot() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("snapshot-rebuild");
    let mut coordinate = build();
    coordinate.manifest = "second/Cargo.toml";
    coordinate.package = "link-second";
    coordinate.bin = "second";
    let first = executables(&root(), &work, &[coordinate]).unwrap();
    let original = std::fs::read(&first[0].1).unwrap();
    coordinate.features = &["link-dep/loud"];
    let second = executables(&root(), &work, &[coordinate]).unwrap();
    assert_ne!(first[0].1, second[0].1);
    assert_ne!(original, std::fs::read(&second[0].1).unwrap());
    assert_eq!(original, std::fs::read(&first[0].1).unwrap());
}

#[test]
fn simultaneous_work_or_output_owner_is_rejected() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("publication-busy");
    std::fs::create_dir_all(&work).unwrap();
    let lock_path = work.with_file_name("publication-busy.vanllia-lock");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .unwrap();
    lock.try_lock().unwrap();
    assert!(
        executables(&root(), &work, &[build()])
            .unwrap_err()
            .contains("busy")
    );
    drop(lock);
    let output = work.join("initrd.img");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(work.join("initrd.img.vanllia-lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(
        build_with(&root(), &work, &output, &[&ENTRY], &GOOD, &[build()])
            .unwrap_err()
            .contains("busy")
    );
}
