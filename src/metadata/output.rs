use super::package::Package;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Output {
    pub packages: Vec<Package>,
    pub workspace_root: std::path::PathBuf,
}
