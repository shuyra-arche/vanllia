use super::Locked;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lock {
    pub schema: u32,
    pub sources: BTreeMap<String, Locked>,
}

impl Default for Lock {
    fn default() -> Self {
        Self {
            schema: 1,
            sources: BTreeMap::new(),
        }
    }
}
