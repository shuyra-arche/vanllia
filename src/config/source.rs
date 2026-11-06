use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub path: Option<PathBuf>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

impl Source {
    pub fn request(&self) -> Result<crate::preparation::Request, String> {
        use crate::preparation::{Reference, Request};
        let choices = usize::from(self.branch.is_some())
            + usize::from(self.tag.is_some())
            + usize::from(self.rev.is_some());
        match (&self.path, &self.git) {
            (Some(path), None) if choices == 0 && !path.as_os_str().is_empty() => {
                Ok(Request::Local(path.clone()))
            }
            (None, Some(url)) if choices <= 1 && !url.is_empty() => {
                let reference = match (&self.branch, &self.tag, &self.rev) {
                    (Some(value), _, _) => Reference::Branch(value.clone()),
                    (_, Some(value), _) => Reference::Tag(value.clone()),
                    (_, _, Some(value)) => Reference::Revision(value.clone()),
                    _ => Reference::Default,
                };
                reference.validate()?;
                Ok(Request::Remote {
                    url: url.clone(),
                    reference,
                })
            }
            _ => {
                Err("source requires either path, or git with at most one of branch/tag/rev".into())
            }
        }
    }
}
