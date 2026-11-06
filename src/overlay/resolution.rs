use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Resolution {
    pub config: PathBuf,
    pub lock: PathBuf,
    pub workspace: PathBuf,
}
