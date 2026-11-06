use super::target::Target;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub(crate) struct Package {
    pub name: String,
    pub id: String,
    pub manifest_path: PathBuf,
    pub targets: Vec<Target>,
    pub source: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
