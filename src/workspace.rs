mod change;
use crate::{Error, config, discovery, error, overlay};
pub use change::Change;
use std::path::{Path, PathBuf};

/// Local source choices persist separately from the product's shared policy and
/// source lock. Optional editor output uses the exact native Cargo overlay.
pub fn configure(
    start: &Path,
    changes: &[Change],
    manifest: Option<&Path>,
    offline: bool,
) -> Result<Option<PathBuf>, Error> {
    let root = discovery::find_root(start)?;
    let _lease = vanllia_build::lease(&root.join(".vanllia/project")).map_err(Error::Workflow)?;
    let mut project = config::project(&root)?;
    let mut overrides = config::overrides(&root)?;
    for change in changes {
        let name = match change {
            Change::Use { name, .. } | Change::Reset(name) => name,
        };
        if !project.sources.contains_key(name) {
            return Err(Error::Workflow(format!("unknown source {name}")));
        }
        match change {
            Change::Use { name, path } => {
                let path = path.canonicalize().map_err(|e| error::io(path, e))?;
                if !path.is_dir() {
                    return Err(Error::Workflow(
                        "workspace source must be a directory".into(),
                    ));
                }
                overrides.sources.insert(name.clone(), path);
            }
            Change::Reset(name) => {
                overrides.sources.remove(name);
            }
        }
    }
    project.overrides = overrides.sources.clone();
    project.validate(&root.join("vanllia.toml"))?;
    let editor = if let Some(manifest) = manifest {
        let manifest = manifest
            .canonicalize()
            .map_err(|e| error::io(manifest, e))?;
        let files = discovery::discover(&root, &project)?;
        if !files.manifests.contains(&manifest) {
            return Err(Error::Workflow(
                "editor manifest is outside the selected source scope".into(),
            ));
        }
        let patches = overlay::discover(&files)?;
        let resolution = overlay::resolve(&root, &manifest, &patches, offline)?;
        let path = resolution.config.with_file_name("editor.json");
        let mut value = serde_json::json!({
            "rust-analyzer.linkedProjects": [resolution.workspace.join("Cargo.toml")],
            "rust-analyzer.cargo.extraArgs": ["--config", resolution.config],
        });
        if let Some(platform) = &project.platform {
            value["rust-analyzer.cargo.target"] = serde_json::json!(platform.target);
        }
        vanllia_build::publish(
            &path,
            &serde_json::to_vec_pretty(&value).map_err(|e| Error::Workflow(e.to_string()))?,
        )
        .map_err(Error::Workflow)?;
        Some(path)
    } else {
        None
    };
    let text =
        toml_edit::ser::to_string_pretty(&overrides).map_err(|e| Error::Workflow(e.to_string()))?;
    vanllia_build::publish(&root.join(".vanllia/overrides.toml"), text.as_bytes())
        .map_err(Error::Workflow)?;
    Ok(editor)
}
