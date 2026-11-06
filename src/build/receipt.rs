use super::Artifact;
use std::{collections::BTreeMap, path::PathBuf};

/// Build evidence only. The guest consumes the binary initrd, never this record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub schema: u32,
    pub platform: crate::config::Platform,
    pub kernel_features: Vec<String>,
    pub builder_sha256: String,
    #[serde(skip)]
    pub directory: PathBuf,
    pub initrd: Artifact,
    pub kernel: Option<Artifact>,
    pub firmware: Option<Artifact>,
    pub images: BTreeMap<String, Artifact>,
    pub locks: BTreeMap<PathBuf, Artifact>,
    pub source_files: BTreeMap<PathBuf, String>,
    pub rustc: String,
    pub cargo: String,
}
