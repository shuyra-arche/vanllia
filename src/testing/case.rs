#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub schema: u32,
    pub name: String,
    pub unit: String,
    #[serde(default = "clean")]
    pub expected: super::Expectation,
    #[serde(default)]
    pub timeout: Option<u64>,
}
fn clean() -> super::Expectation {
    super::Expectation::Clean
}
