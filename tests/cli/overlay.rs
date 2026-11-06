mod support;

use support::{Fixture, git};

#[test]
fn local_patch_is_used_by_real_cargo_without_changing_source_locks() {
    let upstream = Fixture::new();
    upstream.write(
        "Cargo.toml",
        "[package]\nname='helper'\nversion='0.1.0'\nedition='2024'\n[workspace]\n",
    );
    upstream.write("src/lib.rs", "pub fn value() -> u32 { 1 }\n");
    git(&upstream.root, &["init", "-q", "--initial-branch=main"]);
    git(&upstream.root, &["config", "user.name", "Vanllia Test"]);
    git(
        &upstream.root,
        &["config", "user.email", "test@example.invalid"],
    );
    git(&upstream.root, &["add", "."]);
    git(&upstream.root, &["commit", "-qm", "initial"]);

    let local = Fixture::new();
    let url = format!("file://{}", upstream.root.display());
    let cloned = std::process::Command::new("git")
        .args(["clone", "--quiet", "--"])
        .arg(&url)
        .arg(&local.root)
        .output()
        .unwrap();
    assert!(cloned.status.success());
    local.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n");

    let product = Fixture::new();
    product.project();
    product.write("probe/Cargo.toml", &format!("[package]\nname='probe-server'\nversion='0.1.0'\nedition='2024'\n[[bin]]\nname='probe'\npath='src/main.rs'\n[dependencies]\nhelper={{git={url:?}}}\n"));
    product.write(
        "probe/src/main.rs",
        "fn main() { assert_eq!(helper::value(), 2); }\n",
    );
    product.write(
        "vanllia.toml",
        &format!(
            "schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.helper]\npath={:?}\n",
            local.root.to_str().unwrap()
        ),
    );
    let project = vanllia::config::read(&product.root.join("vanllia.toml")).unwrap();
    let files = vanllia::discovery::discover(&product.root, &project).unwrap();
    let patches = vanllia::overlay::discover(&files).unwrap();
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].path, local.root);
    let resolution = vanllia::overlay::resolve(
        &product.root,
        &product.root.join("Cargo.toml"),
        &patches,
        false,
    )
    .unwrap();
    assert!(!product.root.join("Cargo.lock").exists());
    assert!(!local.root.join("Cargo.lock").exists());
    assert!(resolution.lock.is_file());

    let build =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(&product.root)
            .args([
                "build",
                "--locked",
                "--offline",
                "--package",
                "probe-server",
                "--bin",
                "probe",
                "--message-format=json",
                "--target",
                "x86_64-unknown-linux-gnu",
            ])
            .arg("--config")
            .arg(&resolution.config)
            .arg("--target-dir")
            .arg(product.root.join(".vanllia/build"))
            .output()
            .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let executable = String::from_utf8(build.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|message| {
            (message["reason"] == "compiler-artifact" && message["target"]["name"] == "probe")
                .then(|| message["executable"].as_str().map(str::to_string))
                .flatten()
        })
        .unwrap();
    assert!(
        std::process::Command::new(executable)
            .status()
            .unwrap()
            .success()
    );
    assert!(!product.root.join("Cargo.lock").exists());
    assert!(
        std::fs::read_to_string(local.root.join("src/lib.rs"))
            .unwrap()
            .contains("{ 2 }")
    );
}

#[test]
fn an_explicit_repository_conflict_is_rejected_before_patch_selection() {
    let first = Fixture::new();
    first.write(
        "Cargo.toml",
        "[package]\nname='helper'\nversion='0.1.0'\n[workspace]\n",
    );
    first.write("src/lib.rs", "pub fn value() {}\n");
    git(&first.root, &["init", "-q"]);
    git(
        &first.root,
        &["remote", "add", "origin", "https://example.invalid/shared"],
    );
    let second = Fixture::new();
    second.write(
        "Cargo.toml",
        "[package]\nname='helper'\nversion='0.1.0'\n[workspace]\n",
    );
    second.write("src/lib.rs", "pub fn value() {}\n");
    git(&second.root, &["init", "-q"]);
    git(
        &second.root,
        &["remote", "add", "origin", "https://example.invalid/shared"],
    );
    let product = Fixture::new();
    product.project();
    product.write("vanllia.toml", &format!("schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.first]\npath={:?}\n[sources.second]\npath={:?}\n", first.root.to_str().unwrap(), second.root.to_str().unwrap()));
    let project = vanllia::config::read(&product.root.join("vanllia.toml")).unwrap();
    assert!(
        matches!(vanllia::discovery::discover(&product.root, &project), Err(vanllia::Error::Duplicate { kind: "explicit repository", first, second, .. }) if first != second)
    );
}
