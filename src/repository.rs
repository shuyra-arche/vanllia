use crate::{Error, error};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn normalize(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if let Some(path) = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
    {
        format!("https://github.com/{path}")
    } else {
        url.to_string()
    }
}

pub(crate) fn origin(root: &Path) -> Result<Option<String>, Error> {
    if !root.join(".git").exists() {
        return Ok(None);
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| error::io(root, e))?;
    if !output.status.success() {
        return Ok(None);
    }
    let url = String::from_utf8(output.stdout).map_err(|e| Error::Declaration {
        path: root.into(),
        message: e.to_string(),
    })?;
    Ok(Some(normalize(&url)))
}

/// Explicit checkouts replace implicit submodules of the same repository.
/// Two explicit choices remain an error, independent of directory order.
pub(crate) fn overrides(roots: &[PathBuf]) -> Result<BTreeMap<String, PathBuf>, Error> {
    let mut result = BTreeMap::new();
    for root in roots {
        if let Some(origin) = origin(root)?
            && let Some(previous) = result.insert(origin.clone(), root.clone())
            && previous != *root
        {
            return Err(Error::Duplicate {
                kind: "explicit repository",
                id: origin,
                first: previous,
                second: root.clone(),
            });
        }
    }
    Ok(result)
}
