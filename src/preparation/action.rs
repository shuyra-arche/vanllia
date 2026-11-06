use super::{Locked, Reference};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Local(PathBuf),
    Pinned(Locked),
    Resolve { url: String, reference: Reference },
}
