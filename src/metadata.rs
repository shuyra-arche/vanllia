pub(crate) mod output;
mod package;
mod target;

use crate::{Error, error, image::Image};
use output::Output;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

pub fn read(manifests: &BTreeSet<PathBuf>) -> Result<Vec<Image>, Error> {
    let mut observed = BTreeSet::new();
    let mut images = Vec::new();
    for manifest in manifests {
        if observed.contains(manifest) {
            continue;
        }
        let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .current_dir(manifest.parent().unwrap())
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--locked",
                "--offline",
                "--manifest-path",
            ])
            .arg(manifest)
            .output()
            .map_err(|e| error::io(manifest, e))?;
        if !output.status.success() {
            return Err(Error::Cargo {
                manifest: manifest.clone(),
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let metadata: Output =
            serde_json::from_slice(&output.stdout).map_err(|e| Error::Cargo {
                manifest: manifest.clone(),
                message: e.to_string(),
            })?;
        for package in metadata.packages {
            let path = package
                .manifest_path
                .canonicalize()
                .map_err(|e| error::io(&package.manifest_path, e))?;
            if !manifests.contains(&path) || !observed.insert(path.clone()) {
                continue;
            }
            images.extend(
                package
                    .targets
                    .into_iter()
                    .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
                    .map(|target| Image {
                        name: target.name,
                        manifest: path.clone(),
                        workspace: metadata.workspace_root.clone(),
                        package: package.name.clone(),
                        package_id: package.id.clone(),
                    }),
            );
        }
    }
    images.sort_by(|left, right| (&left.name, &left.manifest).cmp(&(&right.name, &right.manifest)));
    Ok(images)
}

pub(crate) fn observe(manifest: &std::path::Path) -> Result<Output, Error> {
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(manifest.parent().unwrap())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
            "--locked",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|e| error::io(manifest, e))?;
    if !output.status.success() {
        return Err(Error::Cargo {
            manifest: manifest.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    serde_json::from_slice(&output.stdout).map_err(|e| Error::Cargo {
        manifest: manifest.to_path_buf(),
        message: e.to_string(),
    })
}
