use super::Reference;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Local(PathBuf),
    Remote { url: String, reference: Reference },
}
