use super::Unit;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Declaration {
    pub path: PathBuf,
    pub unit: Unit,
}
