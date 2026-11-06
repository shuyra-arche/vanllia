use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boot {
    pub system: String,
    #[serde(default)]
    pub kernel: Option<super::Kernel>,
    #[serde(default)]
    pub firmware: Option<super::Input>,
}
