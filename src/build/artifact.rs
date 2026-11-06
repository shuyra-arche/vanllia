#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub path: std::path::PathBuf,
    pub sha256: String,
    pub bytes: u64,
}
