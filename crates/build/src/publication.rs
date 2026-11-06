use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

mod lease;
pub use lease::Lease;

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(crate) fn leases(paths: &[&Path]) -> Result<Vec<Lease>, String> {
    let mut names = Vec::new();
    for path in paths {
        // Cargo work directories follow directory aliases. Publication replaces
        // a file's directory entry, so its final component must stay stable even
        // when that entry starts as a symlink and rename replaces it.
        let resolved = if path.is_dir() {
            path.canonicalize().map_err(|e| e.to_string())?
        } else {
            path.to_path_buf()
        };
        let path = resolved.as_path();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let parent = parent.canonicalize().map_err(|e| e.to_string())?;
        let name = path.file_name().ok_or("lock path needs a file name")?;
        let mut name = name.to_os_string();
        name.push(".vanllia-lock");
        names.push(parent.join(name));
    }
    names.sort();
    names.dedup();
    names
        .into_iter()
        .map(|path| {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| format!("lock {}: {e}", path.display()))?;
            file.try_lock()
                .map_err(|e| format!("busy {}: {e}", path.display()))?;
            // Keep the inode: unlinking a lock file would allow two simultaneous owners.
            Ok(Lease { file })
        })
        .collect()
}

pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary: PathBuf = parent.join(format!(
        ".vanllia-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| format!("publish {}: {e}", path.display()))
    })();
    if temporary.exists() {
        std::fs::remove_file(&temporary)
            .map_err(|e| format!("{result:?}; cleanup {}: {e}", temporary.display()))?;
    }
    result
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
