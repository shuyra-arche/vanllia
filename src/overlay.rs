mod patch;
mod resolution;

pub use patch::Patch;
pub use resolution::Resolution;

use crate::{Error, error, metadata, source::SourceFiles};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

/// Derive Cargo-native patches from the explicitly selected repository roots.
pub fn discover(files: &SourceFiles) -> Result<Vec<Patch>, Error> {
    let mut origins = BTreeMap::new();
    for root in &files.roots {
        if let Some(origin) = crate::repository::origin(root)? {
            origins.insert(root.clone(), origin);
        }
    }
    let mut patches: BTreeMap<(String, String), Patch> = BTreeMap::new();
    let mut observed = BTreeSet::new();
    for manifest in &files.manifests {
        if observed.contains(manifest) {
            continue;
        }
        for package in metadata::observe(manifest)?.packages {
            let path = package
                .manifest_path
                .canonicalize()
                .map_err(|e| error::io(&package.manifest_path, e))?;
            if !files.manifests.contains(&path) || !observed.insert(path.clone()) {
                continue;
            }
            let origin = origins
                .iter()
                .filter(|(root, _)| path.starts_with(root))
                .max_by_key(|(root, _)| root.components().count())
                .map(|(_, origin)| origin);
            let Some(origin) = origin else {
                continue;
            };
            let patch = Patch {
                source: origin.clone(),
                package: package.name,
                path: path.parent().unwrap().to_path_buf(),
            };
            let key = (patch.source.clone(), patch.package.clone());
            if let Some(previous) = patches.get(&key)
                && previous.path != patch.path
            {
                return Err(Error::Duplicate {
                    kind: "local source package",
                    id: patch.package,
                    first: previous.path.clone(),
                    second: patch.path,
                });
            }
            patches.insert(key, patch);
        }
    }
    Ok(patches.into_values().collect())
}

pub fn document(patches: &[Patch]) -> Result<String, Error> {
    let mut document = toml_edit::DocumentMut::new();
    for patch in patches {
        let path = patch.path.to_str().ok_or_else(|| Error::Declaration {
            path: patch.path.clone(),
            message: "Cargo patch path must be UTF-8".into(),
        })?;
        document["patch"][&patch.source][&patch.package]["path"] = toml_edit::value(path);
    }
    Ok(document.to_string())
}

/// Resolve into a separate Cargo-owned lockfile, leaving source checkouts intact.
pub fn resolve(
    root: &Path,
    manifest: &Path,
    patches: &[Patch],
    offline: bool,
) -> Result<Resolution, Error> {
    let observed = metadata::observe(manifest)?;
    let identity = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}\n{}",
                observed.workspace_root.display(),
                document(patches)?
            )
            .as_bytes()
        )
    );
    let directory = root.join(".vanllia/cargo").join(identity);
    std::fs::create_dir_all(&directory).map_err(|e| error::io(&directory, e))?;
    let config = directory.join("overlay.toml");
    let mut document: toml_edit::DocumentMut =
        document(patches)?
            .parse()
            .map_err(|e: toml_edit::TomlError| Error::Declaration {
                path: config.clone(),
                message: e.to_string(),
            })?;
    let lock = directory.join("Cargo.lock");
    let lock_text = lock.to_str().ok_or_else(|| Error::Declaration {
        path: lock.clone(),
        message: "Cargo lock path must be UTF-8".into(),
    })?;
    document["resolver"]["lockfile-path"] = toml_edit::value(lock_text);
    std::fs::write(&config, document.to_string()).map_err(|e| error::io(&config, e))?;
    let source_lock = observed.workspace_root.join("Cargo.lock");
    if !lock.exists() && source_lock.is_file() {
        std::fs::copy(&source_lock, &lock).map_err(|e| error::io(&lock, e))?;
    }
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .current_dir(&observed.workspace_root)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest)
        .arg("--config")
        .arg(&config);
    if offline {
        command.arg("--offline");
    }
    let output = command.output().map_err(|e| error::io(manifest, e))?;
    if !output.status.success() {
        return Err(Error::Cargo {
            manifest: manifest.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let resolved: metadata::output::Output =
        serde_json::from_slice(&output.stdout).map_err(|e| Error::Cargo {
            manifest: manifest.to_path_buf(),
            message: e.to_string(),
        })?;
    for patch in patches {
        for package in resolved
            .packages
            .iter()
            .filter(|package| package.name == patch.package)
        {
            if package.manifest_path.parent() == Some(patch.path.as_path()) {
                continue;
            }
            if package.source.as_deref().is_some_and(|source| {
                source
                    .strip_prefix("git+")
                    .is_some_and(|url| url.split(['?', '#']).next() == Some(patch.source.as_str()))
            }) {
                return Err(Error::Cargo {
                    manifest: manifest.to_path_buf(),
                    message: format!(
                        "Cargo did not use the selected local source for package `{}` at {}",
                        patch.package,
                        patch.path.display()
                    ),
                });
            }
        }
    }
    if !lock.is_file() {
        return Err(Error::Cargo {
            manifest: manifest.to_path_buf(),
            message:
                "Cargo did not produce the configured isolated lockfile; Cargo 1.97+ is required"
                    .into(),
        });
    }
    Ok(Resolution {
        config,
        lock,
        workspace: observed.workspace_root,
    })
}
