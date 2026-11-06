use super::*;

fn repository(name: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("vanllia-source-{name}-{}", std::process::id()));
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    (root, repo)
}

#[test]
fn snapshot_rejects_an_ignored_symlink_ancestor() {
    let (root, repo) = repository("symlink");
    std::fs::create_dir(repo.join("server")).unwrap();
    std::fs::write(repo.join("server/input.txt"), b"tracked\n").unwrap();
    std::fs::write(repo.join(".gitignore"), b"/server\n").unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "--force", "server/input.txt", ".gitignore"])
            .status()
            .unwrap()
            .success()
    );
    std::fs::remove_dir_all(repo.join("server")).unwrap();
    let outside = root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("input.txt"), b"outside\n").unwrap();
    std::os::unix::fs::symlink(&outside, repo.join("server")).unwrap();
    let error = snapshot(&repo, None).unwrap_err();
    assert!(error.contains("symlink"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_uses_prepared_patch_bytes_when_original_file_changes() {
    let (root, repo) = repository("frozen-patch");
    std::fs::write(repo.join("input.txt"), b"before\n").unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["add", "input.txt"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture"
            ])
            .status()
            .unwrap()
            .success()
    );
    let prepared = b"diff --git a/input.txt b/input.txt\n--- a/input.txt\n+++ b/input.txt\n@@ -1 +1 @@\n-before\n+snapshot\n".to_vec();
    std::fs::write(root.join("change.patch"), b"changed after preparation\n").unwrap();
    let mut config: Config =
        serde_json::from_str(include_str!("../../../examples/pipe/vanllia.json")).unwrap();
    config.sources.truncate(1);
    config.sources[0].patches = vec![PathBuf::from("change.patch")];
    let overrides = BTreeMap::from([("pipe".into(), repo.clone())]);
    let attempt = root.join("attempt");
    prepare(
        &config,
        &root,
        &attempt,
        &overrides,
        std::slice::from_ref(&prepared),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(attempt.join("sources/pipe/input.txt")).unwrap(),
        b"snapshot\n"
    );
    assert_eq!(
        std::fs::read(attempt.join("patches/0.patch")).unwrap(),
        prepared
    );
    assert_eq!(
        std::fs::read(root.join("change.patch")).unwrap(),
        b"changed after preparation\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}
