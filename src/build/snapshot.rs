use crate::{Error, error};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn read(roots: &BTreeSet<PathBuf>) -> Result<BTreeMap<PathBuf, String>, Error> {
    let mut result = BTreeMap::new();
    for root in roots {
        if root.join(".git").exists() {
            let output = vanllia_build::workflow::process::capture(
                Command::new("git").arg("-C").arg(root).args([
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                ]),
            )
            .map_err(Error::Workflow)?;
            use std::os::unix::ffi::OsStrExt;
            for name in output
                .split(|byte| *byte == 0)
                .filter(|name| !name.is_empty())
            {
                let relative = Path::new(std::ffi::OsStr::from_bytes(name));
                if relative.components().any(|part| matches!(part, std::path::Component::Normal(name) if name == ".vanllia" || name == "target")) { continue; }
                observe(&root.join(relative), &mut result)?;
            }
        } else {
            walk(root, &mut result)?;
        }
    }
    Ok(result)
}
fn walk(path: &Path, result: &mut BTreeMap<PathBuf, String>) -> Result<(), Error> {
    for entry in std::fs::read_dir(path).map_err(|e| error::io(path, e))? {
        let entry = entry.map_err(|e| error::io(path, e))?;
        if [".git", ".vanllia", "target", "node_modules", ".cache"]
            .iter()
            .any(|skip| entry.file_name() == *skip)
        {
            continue;
        }
        let kind = entry.file_type().map_err(|e| error::io(&entry.path(), e))?;
        if kind.is_dir() {
            if !entry.path().join(".git").exists() {
                walk(&entry.path(), result)?;
            }
        } else {
            observe(&entry.path(), result)?;
        }
    }
    Ok(())
}
fn observe(path: &Path, result: &mut BTreeMap<PathBuf, String>) -> Result<(), Error> {
    let kind = match std::fs::symlink_metadata(path) {
        Ok(kind) => kind,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            result.insert(path.to_path_buf(), "deleted".into());
            return Ok(());
        }
        Err(error) => return Err(error::io(path, error)),
    };
    if kind.is_file() {
        let bytes = std::fs::read(path).map_err(|e| error::io(path, e))?;
        result.insert(path.to_path_buf(), super::digest(&bytes));
    } else if kind.file_type().is_symlink() {
        let target = std::fs::read_link(path).map_err(|e| error::io(path, e))?;
        result.insert(path.to_path_buf(), format!("symlink:{}", target.display()));
    }
    Ok(())
}
