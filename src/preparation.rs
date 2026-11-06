mod action;
mod lock;
mod locked;
mod mode;
mod reference;
mod request;
mod staging;

pub use action::Action;
pub use locked::Locked;
pub use mode::Mode;
pub use reference::Reference;
pub use request::Request;

use crate::{
    Error,
    config::{self, Project},
    discovery, error,
    source_state::SourceState,
};
use lock::Lock;
use sha2::{Digest, Sha256};
use staging::Staging;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Pure source policy. Updates never occur as an accidental cache miss.
pub fn action(request: &Request, locked: Option<&Locked>, mode: Mode) -> Result<Action, String> {
    match request {
        Request::Local(path) => Ok(Action::Local(path.clone())),
        Request::Remote { url, reference } => {
            match (locked, mode) {
                (_, Mode::Update) | (None, Mode::Locked) => Ok(Action::Resolve {
                    url: url.clone(),
                    reference: reference.clone(),
                }),
                (Some(locked), Mode::Locked) => {
                    locked.validate()?;
                    if locked.git != *url || locked.reference != *reference {
                        return Err("source selector changed; run vanllia fetch --update to replace its lock".into());
                    }
                    Ok(Action::Pinned(locked.clone()))
                }
            }
        }
    }
}

pub fn locate(root: &Path, project: &Project) -> Result<Vec<PathBuf>, Error> {
    let lock = read_lock(root)?;
    project
        .sources
        .iter()
        .map(|(name, source)| {
            let request = project
                .request(name, source)
                .map_err(|message| configuration(root, name, message))?;
            match action(&request, lock.sources.get(name), Mode::Locked)
                .map_err(|message| configuration(root, name, message))?
            {
                Action::Local(path) => available(&root.join(path)),
                Action::Pinned(locked) => {
                    let path = cached(root, &locked);
                    validate_checkout(&path, &locked.commit)?;
                    available(&path)
                }
                Action::Resolve { .. } => Err(configuration(
                    root,
                    name,
                    "source has not been resolved; run vanllia fetch".into(),
                )),
            }
        })
        .collect()
}

pub fn prepare(root: &Path, project: &Project, mode: Mode) -> Result<Vec<PathBuf>, Error> {
    project.validate(&root.join("vanllia.toml"))?;
    let _lease = vanllia_build::lease(&root.join(".vanllia/project")).map_err(Error::Workflow)?;
    prepare_locked(root, project, mode)
}

pub(crate) fn prepare_locked(
    root: &Path,
    project: &Project,
    mode: Mode,
) -> Result<Vec<PathBuf>, Error> {
    let previous = read_lock(root)?;
    let mut next = Lock::default();
    let mut roots = Vec::new();
    for (name, source) in &project.sources {
        let request = project
            .request(name, source)
            .map_err(|message| configuration(root, name, message))?;
        let selected = action(&request, previous.sources.get(name), mode)
            .map_err(|message| configuration(root, name, message))?;
        match selected {
            Action::Local(path) => {
                roots.push(available(&root.join(path))?);
                if project.overrides.contains_key(name)
                    && source.git.is_some()
                    && let Some(locked) = previous.sources.get(name)
                {
                    next.sources.insert(name.clone(), locked.clone());
                }
            }
            Action::Pinned(locked) => {
                let path = cached(root, &locked);
                match discovery::source_state(&path)? {
                    SourceState::Available(_) => validate_checkout(&path, &locked.commit)?,
                    SourceState::Missing(_) => {
                        retrieve(root, &locked.git, &locked.reference, Some(&locked.commit))?;
                    }
                }
                roots.push(path);
                next.sources.insert(name.clone(), locked);
            }
            Action::Resolve { url, reference } => {
                let locked = retrieve(root, &url, &reference, None)?;
                roots.push(cached(root, &locked));
                next.sources.insert(name.clone(), locked);
            }
        }
    }
    initialize_submodules(root, &roots)?;
    if next != previous {
        write_lock(root, &next)?;
    }
    Ok(roots)
}

fn configuration(root: &Path, name: &str, message: String) -> Error {
    Error::Declaration {
        path: root.join("vanllia.toml"),
        message: format!("source `{name}`: {message}"),
    }
}

fn read_lock(root: &Path) -> Result<Lock, Error> {
    let path = root.join("vanllia.lock");
    let lock = match std::fs::metadata(&path) {
        Ok(_) => config::read::<Lock>(&path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Lock::default()),
        Err(e) => return Err(error::io(&path, e)),
    };
    if lock.schema != 1 {
        return Err(Error::Declaration {
            path,
            message: "unsupported source lock schema".into(),
        });
    }
    for source in lock.sources.values() {
        source.validate().map_err(|message| Error::Declaration {
            path: path.clone(),
            message,
        })?;
    }
    Ok(lock)
}

fn write_lock(root: &Path, lock: &Lock) -> Result<(), Error> {
    use std::io::Write;
    let path = root.join("vanllia.lock");
    let stage = Staging::new(root)?;
    let temporary = stage.path.join("vanllia.lock");
    let text = toml_edit::ser::to_string_pretty(lock).map_err(|e| Error::Declaration {
        path: path.clone(),
        message: e.to_string(),
    })?;
    let mut file = std::fs::File::create(&temporary).map_err(|e| error::io(&temporary, e))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| error::io(&temporary, e))?;
    std::fs::rename(&temporary, &path).map_err(|e| error::io(&path, e))
}

fn available(path: &Path) -> Result<PathBuf, Error> {
    match discovery::source_state(path)? {
        SourceState::Available(root) => Ok(root),
        SourceState::Missing(path) => Err(Error::SourceMissing(path)),
    }
}

fn cached(root: &Path, locked: &Locked) -> PathBuf {
    let identity = format!("{:x}", Sha256::digest(locked.git.as_bytes()));
    root.join(".vanllia/sources")
        .join(identity)
        .join(&locked.commit)
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|e| error::io(root, e))?;
    if !output.status.success() {
        return Err(Error::Declaration {
            path: root.to_path_buf(),
            message: format!(
                "Git operation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim_end().to_string())
        .map_err(|e| Error::Declaration {
            path: root.to_path_buf(),
            message: e.to_string(),
        })
}

fn validate_checkout(path: &Path, commit: &str) -> Result<(), Error> {
    available(path)?;
    if git(path, &["rev-parse", "--verify", "HEAD"])? != commit {
        return Err(Error::Declaration {
            path: path.to_path_buf(),
            message: "managed source HEAD differs from its locked commit".into(),
        });
    }
    let dirty = git(path, &["status", "--porcelain", "--untracked-files=all"])?;
    if !dirty.is_empty() {
        return Err(Error::Declaration {
            path: path.to_path_buf(),
            message:
                "managed source was modified; use an explicit path source for local development"
                    .into(),
        });
    }
    Ok(())
}

fn retrieve(
    root: &Path,
    url: &str,
    reference: &Reference,
    expected: Option<&str>,
) -> Result<Locked, Error> {
    let stage = Staging::new(root)?;
    let checkout = stage.path.join("checkout");
    let output = Command::new("git")
        .args(["clone", "--no-checkout", "--", url])
        .arg(&checkout)
        .output()
        .map_err(|e| error::io(&checkout, e))?;
    if !output.status.success() {
        return Err(Error::Declaration {
            path: checkout,
            message: format!(
                "Git clone failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }
    let expression = expected
        .map(str::to_string)
        .unwrap_or_else(|| reference.expression());
    let resolve = || {
        git(
            &checkout,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{expression}^{{commit}}"),
            ],
        )
    };
    let commit = match resolve() {
        Ok(commit) => commit,
        Err(_) => {
            let selector = expected
                .map(str::to_string)
                .or_else(|| reference.fetch_ref())
                .ok_or_else(|| {
                    configuration(root, "Git", "cannot resolve default branch".into())
                })?;
            git(&checkout, &["fetch", "--", "origin", &selector])?;
            git(&checkout, &["rev-parse", "--verify", "FETCH_HEAD^{commit}"])?
        }
    };
    let locked = Locked {
        git: url.into(),
        reference: reference.clone(),
        commit,
    };
    locked
        .validate()
        .map_err(|message| configuration(root, "Git", message))?;
    if expected.is_some_and(|expected| expected != locked.commit) {
        return Err(configuration(
            root,
            "Git",
            "fetched source does not match the locked commit".into(),
        ));
    }
    git(&checkout, &["checkout", "--detach", &locked.commit])?;
    let destination = cached(root, &locked);
    std::fs::create_dir_all(destination.parent().unwrap())
        .map_err(|e| error::io(&destination, e))?;
    if destination.exists() {
        validate_checkout(&destination, &locked.commit)?;
    } else {
        std::fs::rename(&checkout, &destination).map_err(|e| error::io(&destination, e))?;
    }
    Ok(locked)
}

fn initialize_submodules(product: &Path, roots: &[PathBuf]) -> Result<(), Error> {
    let overrides = crate::repository::overrides(roots)?;
    let mut pending: BTreeSet<_> = std::iter::once(product.to_path_buf())
        .chain(roots.iter().cloned())
        .collect();
    let mut visited = BTreeSet::new();
    while let Some(root) = pending.pop_first() {
        if !visited.insert(root.clone()) {
            continue;
        }
        for path in discovery::submodule_paths(&root, &overrides)? {
            if !path.join(".git").exists() {
                let relative = path.strip_prefix(&root).unwrap().to_str().ok_or_else(|| {
                    configuration(&root, "submodule", "submodule path must be UTF-8".into())
                })?;
                git(&root, &["submodule", "update", "--init", "--", relative])?;
            }
            let resolved = available(&path)?;
            if !resolved.starts_with(&root)
                && !overrides.values().any(|selected| *selected == resolved)
            {
                return Err(configuration(
                    &root,
                    "submodule",
                    "submodule escapes its source root".into(),
                ));
            }
            pending.insert(resolved);
        }
    }
    Ok(())
}
