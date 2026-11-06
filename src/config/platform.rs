use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub target: String,
    #[serde(default = "profile")]
    pub profile: String,
}
fn profile() -> String {
    "dev".into()
}
