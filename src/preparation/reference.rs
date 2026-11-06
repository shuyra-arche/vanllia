use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Reference {
    Default,
    Branch(String),
    Tag(String),
    Revision(String),
}

impl Reference {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Default => Ok(()),
            Self::Branch(value) | Self::Tag(value) | Self::Revision(value)
                if !value.is_empty() && !value.starts_with('-') && !value.chars().any(char::is_control) => Ok(()),
            _ => Err("Git selector must be nonempty, contain no control characters and not begin with '-'".into()),
        }
    }

    pub(super) fn expression(&self) -> String {
        match self {
            Self::Default => "refs/remotes/origin/HEAD".into(),
            Self::Branch(value) => format!("refs/remotes/origin/{value}"),
            Self::Tag(value) => format!("refs/tags/{value}"),
            Self::Revision(value) => value.clone(),
        }
    }

    pub(super) fn fetch_ref(&self) -> Option<String> {
        match self {
            Self::Default => None,
            Self::Branch(value) => Some(format!("refs/heads/{value}")),
            Self::Tag(value) => Some(format!("refs/tags/{value}")),
            Self::Revision(value) => Some(value.clone()),
        }
    }
}
