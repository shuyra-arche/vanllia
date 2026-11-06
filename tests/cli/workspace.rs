mod support;

use support::{Fixture, git};
use vanllia::{
    config,
    preparation::{self, Mode},
    workspace::{self, Change},
};

#[test]
fn local_choices_preserve_the_shared_git_pin_and_reset_returns_to_it() {
    let upstream = Fixture::new();
    upstream.write(
        "Cargo.toml",
        "[package]\nname='helper'\nversion='0.1.0'\n[workspace]\n",
    );
    upstream.write("src/lib.rs", "pub fn value() {}\n");
    git(&upstream.root, &["init", "-q", "--initial-branch=main"]);
    git(&upstream.root, &["config", "user.name", "Vanllia Test"]);
    git(
        &upstream.root,
        &["config", "user.email", "test@example.invalid"],
    );
    git(&upstream.root, &["add", "."]);
    git(&upstream.root, &["commit", "-qm", "initial"]);
    let product = Fixture::new();
    product.project();
    product.write(
        "vanllia.toml",
        &format!(
            "schema=1\n[boot]\nsystem='system'\n[sources.helper]\ngit={:?}\nbranch='main'\n",
            upstream.root.to_str().unwrap()
        ),
    );
    let original = config::project(&product.root).unwrap();
    let pinned = preparation::prepare(&product.root, &original, Mode::Locked).unwrap();
    let policy = std::fs::read(product.root.join("vanllia.toml")).unwrap();
    let lock = std::fs::read(product.root.join("vanllia.lock")).unwrap();
    upstream.write("src/lib.rs", "pub fn changed() {}\n");
    workspace::configure(
        &product.root,
        &[Change::Use {
            name: "helper".into(),
            path: upstream.root.clone(),
        }],
        None,
        true,
    )
    .unwrap();
    let local = config::project(&product.root).unwrap();
    assert_eq!(
        preparation::prepare(&product.root, &local, Mode::Update).unwrap(),
        std::slice::from_ref(&upstream.root)
    );
    assert_eq!(
        std::fs::read(product.root.join("vanllia.lock")).unwrap(),
        lock
    );
    assert_eq!(
        std::fs::read(product.root.join("vanllia.toml")).unwrap(),
        policy
    );
    assert!(
        std::fs::read_to_string(upstream.root.join("src/lib.rs"))
            .unwrap()
            .contains("changed")
    );
    let overrides = std::fs::read(product.root.join(".vanllia/overrides.toml")).unwrap();
    assert!(
        workspace::configure(
            &product.root,
            &[Change::Reset("unknown".into())],
            None,
            true
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(product.root.join(".vanllia/overrides.toml")).unwrap(),
        overrides
    );
    workspace::configure(&product.root, &[Change::Reset("helper".into())], None, true).unwrap();
    assert_eq!(
        preparation::locate(&product.root, &config::project(&product.root).unwrap()).unwrap(),
        pinned
    );
}

#[test]
fn editor_configuration_uses_the_same_cargo_resolution_and_target() {
    let product = Fixture::new();
    product.project();
    product.write(
        "vanllia.toml",
        "schema=1\n[boot]\nsystem='system'\n[platform]\ntarget='riscv64gc-unknown-none-elf'\n",
    );
    let source = std::fs::read(product.root.join("Cargo.toml")).unwrap();
    let path = workspace::configure(
        &product.root,
        &[],
        Some(&product.root.join("probe/Cargo.toml")),
        true,
    )
    .unwrap()
    .unwrap();
    let settings: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        settings["rust-analyzer.linkedProjects"][0],
        product.root.join("Cargo.toml").to_str().unwrap()
    );
    assert_eq!(
        settings["rust-analyzer.cargo.target"],
        "riscv64gc-unknown-none-elf"
    );
    assert_eq!(settings["rust-analyzer.cargo.extraArgs"][0], "--config");
    let cargo_config = std::path::Path::new(
        settings["rust-analyzer.cargo.extraArgs"][1]
            .as_str()
            .unwrap(),
    );
    assert!(cargo_config.is_file());
    assert!(cargo_config.with_file_name("Cargo.lock").is_file());
    assert_eq!(
        std::fs::read(product.root.join("Cargo.toml")).unwrap(),
        source
    );
    assert!(!product.root.join("Cargo.lock").exists());
}
