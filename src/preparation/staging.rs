use crate::{Error, error};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) struct Staging {
    pub path: PathBuf,
}

impl Staging {
    pub fn new(root: &Path) -> Result<Self, Error> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let parent = root.join(".vanllia/tmp");
        std::fs::create_dir_all(&parent).map_err(|e| error::io(&parent, e))?;
        loop {
            let path = parent.join(format!(
                "source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(error::io(&path, e)),
            }
        }
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
