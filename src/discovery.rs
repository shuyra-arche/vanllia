use crate::{Error, config::Project, error, source::SourceFiles, source_state::SourceState};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn find_root(start: &Path) -> Result<PathBuf, Error> {
    let start = start.canonicalize().map_err(|e| error::io(start, e))?;
    let directory = if start.is_file() {
        start.parent().unwrap()
    } else {
        &start
    };
    directory
        .ancestors()
        .find(|root| root.join("vanllia.toml").is_file())
        .map(Path::to_path_buf)
        .ok_or(Error::RootMissing(start))
}

pub fn source_state(path: &Path) -> Result<SourceState, Error> {
    match path.canonicalize() {
        Ok(root) if root.is_dir() => Ok(SourceState::Available(root)),
        Ok(_) => Err(Error::Declaration {
            path: path.to_path_buf(),
            message: "source must be a directory".into(),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(SourceState::Missing(path.to_path_buf()))
        }
        Err(e) => Err(error::io(path, e)),
    }
}

pub fn discover(root: &Path, project: &Project) -> Result<SourceFiles, Error> {
    let explicit = crate::preparation::locate(root, project)?;
    let overrides = crate::repository::overrides(&explicit)?;
    let mut pending = BTreeSet::from([root.to_path_buf()]);
    pending.extend(explicit);
    let mut visited = BTreeSet::new();
    let mut files = SourceFiles::default();
    while let Some(source) = pending.pop_first() {
        if !visited.insert(source.clone()) {
            continue;
        }
        files.roots.insert(source.clone());
        pending.extend(submodules(&source, &overrides)?);
        if source.join(".git").exists() {
            git_files(&source, &mut files)?;
        } else {
            walk(&source, &source, &mut files)?;
        }
    }
    Ok(files)
}

fn excluded(path: &Path) -> bool {
    path.components().any(|part| matches!(part, Component::Normal(name) if
        [".git", "target", ".vanllia", ".cache", "node_modules", "vendor"].iter().any(|skip| name == *skip)))
}

fn record(path: PathBuf, files: &mut SourceFiles) {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("unit.toml") => {
            files.units.insert(path);
        }
        Some("test.toml") => {
            files.tests.insert(path);
        }
        Some("Cargo.toml") => {
            files.manifests.insert(path);
        }
        _ => {}
    }
}

fn walk(root: &Path, directory: &Path, files: &mut SourceFiles) -> Result<(), Error> {
    if directory != root && directory.join(".git").exists() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(directory)
        .map_err(|e| error::io(directory, e))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| error::io(directory, e))?;
    entries.sort();
    for path in entries {
        if excluded(path.strip_prefix(root).unwrap()) {
            continue;
        }
        let kind = std::fs::symlink_metadata(&path).map_err(|e| error::io(&path, e))?;
        if kind.is_dir() {
            walk(root, &path, files)?;
        } else if kind.is_file() {
            record(path, files);
        }
    }
    Ok(())
}

fn git_files(root: &Path, files: &mut SourceFiles) -> Result<(), Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .map_err(|e| error::io(root, e))?;
    if !output.status.success() {
        return Err(Error::Declaration {
            path: root.to_path_buf(),
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
    {
        let name = std::str::from_utf8(bytes).map_err(|e| Error::Declaration {
            path: root.to_path_buf(),
            message: e.to_string(),
        })?;
        let relative = Path::new(name);
        if excluded(relative) {
            continue;
        }
        let mut path = root.to_path_buf();
        let mut regular = true;
        for part in relative.components() {
            if !matches!(part, Component::Normal(_)) {
                regular = false;
                break;
            }
            path.push(part);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_symlink() => {
                    regular = false;
                    break;
                }
                Ok(metadata) if metadata.is_dir() && path.join(".git").exists() => {
                    regular = false;
                    break;
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    regular = false;
                    break;
                }
                Err(e) => return Err(error::io(&path, e)),
            }
        }
        if regular && path.is_file() {
            record(path, files);
        }
    }
    Ok(())
}

pub(crate) fn submodule_paths(
    root: &Path,
    overrides: &BTreeMap<String, PathBuf>,
) -> Result<Vec<PathBuf>, Error> {
    let config = root.join(".gitmodules");
    if !config.is_file() {
        return Ok(Vec::new());
    }
    let output = Command::new("git")
        .args(["config", "--file"])
        .arg(&config)
        .args(["--get-regexp", r"^submodule\..*\.path$"])
        .output()
        .map_err(|e| error::io(&config, e))?;
    if output.status.code() == Some(1) {
        return Ok(Vec::new());
    }
    if !output.status.success() {
        return Err(Error::Declaration {
            path: config,
            message: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let text = String::from_utf8(output.stdout).map_err(|e| Error::Declaration {
        path: config.clone(),
        message: e.to_string(),
    })?;
    text.lines()
        .map(|line| {
            let (key, name) = line.split_once(' ').ok_or_else(|| Error::Declaration {
                path: config.clone(),
                message: "invalid submodule path record".into(),
            })?;
            let relative = Path::new(name);
            if relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(Error::Declaration {
                    path: config.clone(),
                    message: format!("submodule path must stay inside its source root: {name}"),
                });
            }
            let path = root.join(relative);
            if !overrides.is_empty() {
                let stem = key
                    .strip_suffix(".path")
                    .ok_or_else(|| Error::Declaration {
                        path: config.clone(),
                        message: "invalid submodule key".into(),
                    })?;
                let url = Command::new("git")
                    .args(["config", "--file"])
                    .arg(&config)
                    .arg("--get")
                    .arg(format!("{stem}.url"))
                    .output()
                    .map_err(|e| error::io(&config, e))?;
                if url.status.success() {
                    let url = String::from_utf8(url.stdout).map_err(|e| Error::Declaration {
                        path: config.clone(),
                        message: e.to_string(),
                    })?;
                    if let Some(selected) = overrides.get(&crate::repository::normalize(&url)) {
                        return Ok(selected.clone());
                    }
                }
            }
            Ok(path)
        })
        .collect()
}

fn submodules(root: &Path, overrides: &BTreeMap<String, PathBuf>) -> Result<Vec<PathBuf>, Error> {
    submodule_paths(root, overrides)?
        .into_iter()
        .map(|path| {
            if !path.join(".git").exists() {
                return Err(Error::SourceMissing(path));
            }
            let resolved = path.canonicalize().map_err(|e| error::io(&path, e))?;
            if !resolved.starts_with(root)
                && !overrides.values().any(|selected| *selected == resolved)
            {
                return Err(Error::Declaration {
                    path: root.join(".gitmodules"),
                    message: format!("submodule escapes source root: {}", path.display()),
                });
            }
            Ok(resolved)
        })
        .collect()
}
