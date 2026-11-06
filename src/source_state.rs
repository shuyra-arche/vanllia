use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq)]
pub enum SourceState {
    Available(PathBuf),
    Missing(PathBuf),
}
