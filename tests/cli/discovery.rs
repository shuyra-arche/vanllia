mod support;

use support::{Fixture, git};
use vanllia::{Error, Selection, config, discovery};

#[test]
fn real_cargo_metadata_binds_bins_without_compiling_or_writing_a_lock() {
    let fixture = Fixture::new();
    fixture.project();
    fixture.write(
        "probe/build.rs",
        "fn main() { panic!(\"metadata must never execute this build script\"); }\n",
    );
    let catalog = vanllia::load(&fixture.root.join("probe/src")).unwrap();
    assert_eq!(catalog.images()["probe"].package, "probe-server");
    let plan = vanllia::plan(&catalog, &Selection::Default).unwrap();
    assert_eq!(plan.order, ["system", "probe"]);
    assert!(!fixture.root.join("Cargo.lock").exists());
    assert!(!fixture.root.join("target").exists());
}

#[test]
fn discovery_skips_output_symlinks_and_undeclared_nested_repositories() {
    let fixture = Fixture::new();
    fixture.project();
    fixture.write("target/unit.toml", "invalid");
    fixture.write("nested/unit.toml", "invalid");
    git(&fixture.root.join("nested"), &["init", "-q"]);
    std::os::unix::fs::symlink(fixture.root.join("unused"), fixture.root.join("linked")).unwrap();
    let project = config::read(&fixture.root.join("vanllia.toml")).unwrap();
    let files = discovery::discover(&fixture.root, &project).unwrap();
    assert_eq!(files.units.len(), 3);
    assert_eq!(vanllia::load(&fixture.root).unwrap().units().len(), 3);
}

#[test]
fn git_ignore_rules_exclude_custom_build_caches() {
    let fixture = Fixture::new();
    fixture.project();
    git(&fixture.root, &["init", "-q"]);
    fixture.write(".gitignore", "/custom-output/\n");
    fixture.write("custom-output/unit.toml", "invalid");
    assert_eq!(vanllia::load(&fixture.root).unwrap().units().len(), 3);
}

#[test]
fn parent_tracking_does_not_claim_an_undeclared_nested_git_repository() {
    let fixture = Fixture::new();
    fixture.project();
    git(&fixture.root, &["init", "-q"]);
    fixture.write("nested/unit.toml", "invalid");
    git(&fixture.root, &["add", "nested/unit.toml"]);
    git(&fixture.root.join("nested"), &["init", "-q"]);
    assert_eq!(vanllia::load(&fixture.root).unwrap().units().len(), 3);
}

#[test]
fn an_explicit_local_source_is_discovered_without_merging_its_product_defaults() {
    let fixture = Fixture::new();
    fixture.project();
    fixture.write(
        "vanllia.toml",
        "schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.peer]\npath='peer'\n",
    );
    fixture.binary("peer", "peer");
    fixture.write("peer/Cargo.toml", "[package]\nname='peer-server'\nversion='0.1.0'\n[[bin]]\nname='peer'\npath='src/main.rs'\n[workspace]\n");
    fixture.write(
        "peer/vanllia.toml",
        "schema=1\nunits=['not-a-unit']\n[boot]\nsystem='different'\n",
    );
    git(&fixture.root.join("peer"), &["init", "-q"]);
    let catalog = vanllia::load(&fixture.root).unwrap();
    assert!(catalog.units().contains_key("peer"));
    assert_eq!(
        vanllia::plan(&catalog, &Selection::Default).unwrap().order,
        ["system", "probe"]
    );
}

#[test]
fn missing_sources_and_unknown_fields_are_errors_with_provenance() {
    let fixture = Fixture::new();
    fixture.project();
    fixture.write(
        "vanllia.toml",
        "schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.peer]\npath='missing'\n",
    );
    assert!(
        matches!(vanllia::load(&fixture.root), Err(Error::SourceMissing(path)) if path.ends_with("missing"))
    );
    fixture.write(
        "vanllia.toml",
        "schema=1\nunits=['probe']\ncluser='typo'\n[boot]\nsystem='system'\n",
    );
    let error = vanllia::load(&fixture.root).unwrap_err().to_string();
    assert!(error.contains("vanllia.toml") && error.contains("cluser"));
}

#[test]
fn uninitialized_submodules_are_not_silently_treated_as_empty_sources() {
    let fixture = Fixture::new();
    fixture.project();
    fixture.write(".gitmodules", "[submodule \"missing\"]\npath = components/missing\nurl = https://example.invalid/missing.git\n");
    assert!(
        matches!(vanllia::load(&fixture.root), Err(Error::SourceMissing(path)) if path.ends_with("components/missing"))
    );
}

#[test]
fn an_explicit_checkout_replaces_an_uninitialized_submodule_without_fetching_it() {
    let fixture = Fixture::new();
    fixture.project();
    let local = Fixture::new();
    local.binary("provider", "provider");
    git(&local.root, &["init", "-q"]);
    git(
        &local.root,
        &[
            "remote",
            "add",
            "origin",
            "git@github.com:example/provider.git",
        ],
    );
    fixture.write(".gitmodules", "[submodule \"provider\"]\npath=components/provider\nurl=https://github.com/example/provider.git\n");
    fixture.write(
        "vanllia.toml",
        &format!(
            "schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.provider]\npath={:?}\n",
            local.root.to_str().unwrap()
        ),
    );
    let project = config::read(&fixture.root.join("vanllia.toml")).unwrap();
    vanllia::preparation::prepare(&fixture.root, &project, vanllia::preparation::Mode::Locked)
        .unwrap();
    let files = discovery::discover(&fixture.root, &project).unwrap();
    assert!(files.roots.contains(&local.root));
    assert!(!fixture.root.join("components/provider").exists());
    assert_eq!(vanllia::load(&fixture.root).unwrap().units().len(), 4);
    assert_eq!(
        vanllia::overlay::discover(&files).unwrap()[0].source,
        "https://github.com/example/provider.git"
    );
}

#[test]
fn cli_plan_works_from_a_component_and_does_not_save_a_deployment() {
    let fixture = Fixture::new();
    fixture.project();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vanllia"))
        .current_dir(fixture.root.join("probe/src"))
        .args(["plan", "--unit", "unused"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("unit: unused") && text.contains("package unused-server"));
    assert!(!text.contains("unit: probe"));
    assert!(!fixture.root.join("Cargo.lock").exists());
    assert!(!fixture.root.join("deployment.toml").exists());
}
