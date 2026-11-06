/// Exclusive ownership ends at this guard, even if a concurrent fork inherited
/// the file descriptor before its close-on-exec flag takes effect.
pub struct Lease {
    pub(crate) file: std::fs::File,
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Err(error) = self.file.unlock() {
            eprintln!("lease unlock failed: {error}");
        }
    }
}
