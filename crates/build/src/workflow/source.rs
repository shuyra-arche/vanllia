use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use super::{config::Config, process::capture};
use crate::publication;

// Resolve existing ancestors before comparing paths, including a not-yet-created
// state reached through a symlink. This function never creates directories.
pub(super) fn resolved(path: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    for ancestor in absolute.ancestors() {
        match ancestor.canonicalize() {
            Ok(mut root) => {
                for component in absolute.strip_prefix(ancestor).unwrap().components() {
                    match component {
                        Component::Normal(name) => root.push(name),
                        Component::ParentDir => {
                            root.pop();
                        }
                        Component::CurDir => {}
                        _ => return Err("invalid state path".into()),
                    }
                }
                return Ok(root);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Err(format!("cannot resolve {}", path.display()))
}

pub(super) fn separate_state(state: &Path, input: &Path) -> Result<(), String> {
    let input = resolved(input)?;
    if state.starts_with(&input) {
        return Err(format!(
            "state {} must be outside input checkout {}",
            state.display(),
            input.display()
        ));
    }
    Ok(())
}

pub(super) fn relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(format!(
            "expected contained relative path: {}",
            path.display()
        ));
    }
    Ok(())
}

pub(super) fn snapshot(
    root: &Path,
    destination: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let listed = capture(Command::new("git").arg("-C").arg(root).args([
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ]))?;
    let mut files = BTreeMap::new();
    for name in listed.split(|byte| *byte == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(name).map_err(|e| e.to_string())?;
        relative(Path::new(name))?;
        let mut component_path = root.to_path_buf();
        for component in Path::new(name).components() {
            component_path.push(component);
            match component_path.symlink_metadata() {
                Ok(metadata) if metadata.is_symlink() => {
                    return Err(format!(
                        "source snapshot rejects symlink {}",
                        component_path.display()
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        let path = root.join(name);
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        if metadata.is_dir() {
            continue;
        } // gitlinks are independent declared sources
        if !metadata.is_file() {
            return Err(format!(
                "source snapshot rejects symlink/special file {name}"
            ));
        }
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        use std::os::unix::fs::PermissionsExt;
        files.insert(
            name.to_string(),
            serde_json::json!({"sha256": publication::digest(&bytes),
            "executable": metadata.permissions().mode() & 0o111}),
        );
        if let Some(destination) = destination {
            let output = destination.join(name);
            publication::write(&output, &bytes)?;
            std::fs::set_permissions(&output, metadata.permissions()).map_err(|e| e.to_string())?;
        }
    }
    Ok(serde_json::json!(files))
}

pub(super) fn inventory(root: &Path) -> Result<serde_json::Value, String> {
    fn visit(
        root: &Path,
        path: &Path,
        files: &mut BTreeMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            if name == ".git" || name == "target" {
                continue;
            }
            let metadata = entry.path().symlink_metadata().map_err(|e| e.to_string())?;
            if metadata.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if metadata.is_file() {
                use std::os::unix::fs::PermissionsExt;
                let bytes = std::fs::read(entry.path()).map_err(|e| e.to_string())?;
                files.insert(entry.path().strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    serde_json::json!({"sha256": publication::digest(&bytes), "executable": metadata.permissions().mode() & 0o111}));
            } else {
                return Err(format!(
                    "untracked symlink/special input {}",
                    entry.path().display()
                ));
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(serde_json::json!(files))
}

pub(super) fn prepare(
    config: &Config,
    base: &Path,
    attempt: &Path,
    overrides: &BTreeMap<String, PathBuf>,
    patch_inputs: &[Vec<u8>],
) -> Result<(), String> {
    if patch_inputs.len()
        != config
            .sources
            .iter()
            .map(|s| s.patches.len())
            .sum::<usize>()
    {
        return Err("prepared patch input count mismatch".into());
    }
    let mut patch_inputs = patch_inputs.iter().enumerate();
    let sources = attempt.join("sources");
    std::fs::create_dir_all(&sources).map_err(|e| e.to_string())?;
    let mut evidence = Vec::new();
    for source in &config.sources {
        if source.rev.len() != 40 || !source.rev.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("{} requires a full fixed commit", source.name));
        }
        relative(Path::new(&source.name))?;
        if Path::new(&source.name).components().count() != 1 {
            return Err("source names must be one path component".into());
        }
        let root = sources.join(&source.name);
        if overrides.contains_key(&source.name) && !source.submodules.is_empty() {
            return Err("local overrides with gitlinks require separately declared sources".into());
        }
        let (head, local) = if let Some(local) = overrides.get(&source.name) {
            std::fs::create_dir(&root).map_err(|e| e.to_string())?;
            let files = snapshot(local, Some(&root))?;
            let head = capture(
                Command::new("git")
                    .arg("-C")
                    .arg(local)
                    .args(["rev-parse", "HEAD"]),
            )?;
            (
                String::from_utf8_lossy(&head).trim().to_string(),
                Some(files),
            )
        } else {
            capture(
                Command::new("git")
                    .args(["clone", "--no-checkout", "--"])
                    .arg(&source.git)
                    .arg(&root),
            )?;
            capture(Command::new("git").arg("-C").arg(&root).args([
                "checkout",
                "--detach",
                &source.rev,
            ]))?;
            let head = capture(
                Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .args(["rev-parse", "HEAD"]),
            )?;
            let head = String::from_utf8_lossy(&head).trim().to_string();
            if head != source.rev {
                return Err(format!("source {} commit mismatch", source.name));
            }
            (head, None)
        };
        let submodules = if source.submodules.is_empty() {
            String::new()
        } else {
            for path in &source.submodules {
                relative(path)?;
            }
            capture(
                Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .args(["submodule", "update", "--init", "--recursive", "--"])
                    .args(&source.submodules),
            )?;
            let status = capture(Command::new("git").arg("-C").arg(&root).args([
                "submodule",
                "status",
                "--recursive",
            ]))?;
            let status = String::from_utf8(status).map_err(|e| e.to_string())?;
            if status
                .lines()
                .any(|line| line.starts_with('+') || line.starts_with('U'))
            {
                return Err("submodule commit mismatch".into());
            }
            status
        };
        let mut patches = Vec::new();
        for patch in &source.patches {
            let (index, bytes) = patch_inputs.next().ok_or("missing prepared patch input")?;
            let saved = attempt.join("patches").join(format!("{index}.patch"));
            publication::write(&saved, bytes)?;
            capture(
                Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .arg("apply")
                    .arg(&saved),
            )?;
            patches.push(
                serde_json::json!({"file": base.join(patch), "snapshot": saved,
                "sha256": publication::digest(bytes)}),
            );
        }
        evidence.push(serde_json::json!({"name": source.name, "git": source.git,
            "baseline": source.rev, "head": head, "override": overrides.get(&source.name),
            "snapshot": local, "patches": patches, "gitlinks": submodules}));
    }
    publication::write(
        &attempt.join("sources.json"),
        &serde_json::to_vec_pretty(&evidence).unwrap(),
    )
}

#[cfg(test)]
mod tests;

pub(super) fn configure(config: &Config, base: &Path, attempt: &Path) -> Result<(), String> {
    let sources = attempt.join("sources");
    // The existing caller is migrated in this private development workspace.
    // Its lock is re-resolved here; shared/user lockfiles never need restoration.
    let host_manifest = sources
        .join(&config.host.source)
        .join(&config.host.dependency_manifest);
    let mut host: toml_edit::DocumentMut = std::fs::read_to_string(&host_manifest)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e| format!("manifest: {e}"))?;
    let dependencies = host["dependencies"]
        .as_table_mut()
        .ok_or("host dependencies missing")?;
    dependencies.remove("image-builder");
    dependencies.remove(&config.host.dependency_alias);
    let library_source = base
        .join(&config.library)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let library = attempt.join("library");
    std::fs::create_dir(&library).map_err(|e| e.to_string())?;
    snapshot(&library_source, Some(&library))?;
    host["dependencies"][&config.host.dependency_alias]["package"] =
        toml_edit::value("vanllia-build");
    host["dependencies"][&config.host.dependency_alias]["path"] =
        toml_edit::value(library.to_string_lossy().as_ref());
    publication::write(&host_manifest, host.to_string().as_bytes())?;
    let mut locks = Vec::new();
    for workspace in &config.workspaces {
        relative(&workspace.manifest)?;
        let manifest = sources.join(&workspace.source).join(&workspace.manifest);
        let mut doc: toml_edit::DocumentMut = std::fs::read_to_string(&manifest)
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|e| format!("manifest: {e}"))?;
        for (name, package) in &workspace.packages {
            relative(&package.path)?;
            let provider = config
                .sources
                .iter()
                .find(|s| s.name == package.source)
                .ok_or_else(|| format!("unknown provider {}", package.source))?;
            let path = sources.join(&package.source).join(&package.path);
            if !path.join("Cargo.toml").is_file() {
                return Err(format!("missing package {}", path.display()));
            }
            doc["patch"][&provider.git][name]["path"] =
                toml_edit::value(path.to_string_lossy().as_ref());
        }
        publication::write(&manifest, doc.to_string().as_bytes())?;
        let before = manifest.parent().unwrap().join("Cargo.lock");
        let original = std::fs::read(&before)
            .map_err(|e| format!("fixed workspace needs Cargo.lock {}: {e}", before.display()))?;
        let metadata = capture(
            super::cargo(config)
                .args(["metadata", "--format-version", "1"])
                .arg("--manifest-path")
                .arg(&manifest)
                .current_dir(manifest.parent().unwrap()),
        )?;
        let metadata: serde_json::Value =
            serde_json::from_slice(&metadata).map_err(|e| e.to_string())?;
        for (name, package) in &workspace.packages {
            let expected = sources
                .join(&package.source)
                .join(&package.path)
                .join("Cargo.toml")
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let found: Vec<_> = metadata["packages"]
                .as_array()
                .ok_or("packages missing")?
                .iter()
                .filter(|p| p["name"] == name.as_str())
                .collect();
            if found.len() != 1
                || found[0]["source"] != serde_json::Value::Null
                || Path::new(found[0]["manifest_path"].as_str().unwrap_or(""))
                    .canonicalize()
                    .ok()
                    .as_ref()
                    != Some(&expected)
            {
                return Err(format!(
                    "override/source verification failed for {name} in {}",
                    manifest.display()
                ));
            }
        }
        publication::write(
            &attempt.join(format!("metadata-{}.json", locks.len())),
            &serde_json::to_vec_pretty(&metadata).unwrap(),
        )?;
        let resolved = std::fs::read(&before).map_err(|e| e.to_string())?;
        locks.push(serde_json::json!({"manifest": manifest, "baseline_sha256": publication::digest(&original),
            "development_sha256": publication::digest(&resolved)}));
    }
    publication::write(
        &attempt.join("locks.json"),
        &serde_json::to_vec_pretty(&locks).unwrap(),
    )
}
