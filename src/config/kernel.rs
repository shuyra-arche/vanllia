#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kernel {
    pub image: String,
    #[serde(default)]
    pub features: Vec<String>,
}
