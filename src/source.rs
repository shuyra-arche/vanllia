use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct SourceFiles {
    pub roots: BTreeSet<PathBuf>,
    pub units: BTreeSet<PathBuf>,
    pub tests: BTreeSet<PathBuf>,
    pub manifests: BTreeSet<PathBuf>,
}
