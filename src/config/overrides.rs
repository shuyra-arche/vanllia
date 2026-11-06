use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    pub schema: u32,
    pub sources: BTreeMap<String, PathBuf>,
}
impl Default for Overrides {
    fn default() -> Self {
        Self {
            schema: 1,
            sources: BTreeMap::new(),
        }
    }
}
