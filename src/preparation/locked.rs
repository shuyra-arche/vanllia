use super::Reference;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Locked {
    pub git: String,
    pub reference: Reference,
    pub commit: String,
}

impl Locked {
    pub(super) fn validate(&self) -> Result<(), String> {
        self.reference.validate()?;
        if self.git.is_empty()
            || !matches!(self.commit.len(), 40 | 64)
            || !self.commit.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("source lock requires a Git URL and a full commit identity".into());
        }
        Ok(())
    }
}
