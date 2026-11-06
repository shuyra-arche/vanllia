#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[serde(default)]
    pub path: Option<std::path::PathBuf>,
    #[serde(default)]
    pub url: Option<String>,
    pub sha256: String,
    pub bytes: u64,
}
