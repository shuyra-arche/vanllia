use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Image {
    pub name: String,
    pub manifest: PathBuf,
    pub workspace: PathBuf,
    pub package: String,
    pub package_id: String,
}
