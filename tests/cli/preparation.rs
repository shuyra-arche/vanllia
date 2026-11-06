mod support;

use support::{Fixture, git};
use vanllia::{
    config,
    preparation::{self, Action, Locked, Mode, Reference, Request},
};

fn remote() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname='provider'\nversion='0.1.0'\n[[bin]]\nname='provider'\npath='src/main.rs'\n[workspace]\n");
    fixture.write("src/main.rs", "fn main() {}\n");
    fixture.write(
        "unit.toml",
        "schema=1\nid='provider'\n[loader]\nimage='provider'\n",
    );
    git(&fixture.root, &["init", "-q", "--initial-branch=main"]);
    git(&fixture.root, &["config", "user.name", "Vanllia Test"]);
    git(
        &fixture.root,
        &["config", "user.email", "test@example.invalid"],
    );
    git(&fixture.root, &["add", "."]);
    git(&fixture.root, &["commit", "-qm", "initial"]);
    fixture
}

fn project(fixture: &Fixture, upstream: &Fixture, selector: &str) -> config::Project {
    fixture.project();
    fixture.write("vanllia.toml", &format!("schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.provider]\ngit={:?}\n{selector}\n", upstream.root.to_str().unwrap()));
    config::read(&fixture.root.join("vanllia.toml")).unwrap()
}

#[test]
fn source_policy_is_pure_and_locked_updates_are_explicit() {
    let request = Request::Remote {
        url: "https://example.invalid/source".into(),
        reference: Reference::Branch("main".into()),
    };
    let locked = Locked {
        git: "https://example.invalid/source".into(),
        reference: Reference::Branch("main".into()),
        commit: "a".repeat(40),
    };
    assert!(matches!(
        preparation::action(&request, Some(&locked), Mode::Locked).unwrap(),
        Action::Pinned(_)
    ));
    assert!(matches!(
        preparation::action(&request, Some(&locked), Mode::Update).unwrap(),
        Action::Resolve { .. }
    ));
    let different = Request::Remote {
        url: "https://example.invalid/source".into(),
        reference: Reference::Tag("v1".into()),
    };
    assert!(preparation::action(&different, Some(&locked), Mode::Locked).is_err());
}

#[test]
fn branch_resolution_is_cached_and_remains_pinned_until_update() {
    let upstream = remote();
    let fixture = Fixture::new();
    let project = project(&fixture, &upstream, "branch='main'");
    let roots = preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap();
    let original = std::fs::read(fixture.root.join("vanllia.lock")).unwrap();
    assert_eq!(roots, preparation::locate(&fixture.root, &project).unwrap());
    assert!(
        vanllia::load(&fixture.root)
            .unwrap()
            .units()
            .contains_key("provider")
    );
    upstream.write("revision", "second");
    git(&upstream.root, &["add", "."]);
    git(&upstream.root, &["commit", "-qm", "second"]);
    assert_eq!(
        preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap(),
        roots
    );
    assert_eq!(
        std::fs::read(fixture.root.join("vanllia.lock")).unwrap(),
        original
    );
    let updated = preparation::prepare(&fixture.root, &project, Mode::Update).unwrap();
    assert_ne!(updated, roots);
    assert_ne!(
        std::fs::read(fixture.root.join("vanllia.lock")).unwrap(),
        original
    );
}

#[test]
fn tags_and_remote_refs_resolve_to_commits() {
    let upstream = remote();
    git(&upstream.root, &["tag", "v1"]);
    git(&upstream.root, &["update-ref", "refs/pull/1/head", "HEAD"]);
    for selector in ["tag='v1'", "rev='refs/pull/1/head'"] {
        let fixture = Fixture::new();
        let project = project(&fixture, &upstream, selector);
        let roots = preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap();
        assert!(roots[0].join("unit.toml").is_file());
        assert_eq!(preparation::locate(&fixture.root, &project).unwrap(), roots);
    }
}

#[test]
fn a_cached_checkout_works_without_the_remote_and_detects_mutations() {
    let upstream = remote();
    let fixture = Fixture::new();
    let project = project(&fixture, &upstream, "");
    let roots = preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap();
    std::fs::remove_dir_all(&upstream.root).unwrap();
    assert_eq!(
        preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap(),
        roots
    );
    std::fs::write(roots[0].join("unit.toml"), "tampered").unwrap();
    assert!(preparation::prepare(&fixture.root, &project, Mode::Locked).is_err());
}

#[test]
fn failed_updates_preserve_the_previous_source_lock() {
    let upstream = remote();
    let fixture = Fixture::new();
    let project = project(&fixture, &upstream, "branch='main'");
    preparation::prepare(&fixture.root, &project, Mode::Locked).unwrap();
    let previous = std::fs::read(fixture.root.join("vanllia.lock")).unwrap();
    std::fs::remove_dir_all(&upstream.root).unwrap();
    assert!(preparation::prepare(&fixture.root, &project, Mode::Update).is_err());
    assert_eq!(
        std::fs::read(fixture.root.join("vanllia.lock")).unwrap(),
        previous
    );
}

#[test]
fn ambiguous_source_selectors_fail_before_git_is_run() {
    let upstream = remote();
    let fixture = Fixture::new();
    let project = project(&fixture, &upstream, "branch='main'\ntag='v1'");
    assert!(
        project
            .validate(&fixture.root.join("vanllia.toml"))
            .is_err()
    );
    assert!(preparation::prepare(&fixture.root, &project, Mode::Locked).is_err());
    assert!(!fixture.root.join(".vanllia").exists());
}

#[test]
fn fetching_a_local_source_keeps_its_uncommitted_changes() {
    let upstream = remote();
    upstream.write("src/main.rs", "fn main() { /* local change */ }\n");
    let fixture = Fixture::new();
    fixture.project();
    fixture.write(
        "vanllia.toml",
        &format!(
            "schema=1\nunits=['probe']\n[boot]\nsystem='system'\n[sources.provider]\npath={:?}\n",
            upstream.root.to_str().unwrap()
        ),
    );
    let project = config::read(&fixture.root.join("vanllia.toml")).unwrap();
    assert_eq!(
        preparation::prepare(&fixture.root, &project, Mode::Update).unwrap(),
        std::slice::from_ref(&upstream.root)
    );
    assert!(
        std::fs::read_to_string(upstream.root.join("src/main.rs"))
            .unwrap()
            .contains("local change")
    );
    assert!(!fixture.root.join("vanllia.lock").exists());
}
