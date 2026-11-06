use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Patch {
    pub source: String,
    pub package: String,
    pub path: PathBuf,
}
