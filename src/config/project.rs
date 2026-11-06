use super::{Boot, Source};
use crate::Error;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema: u32,
    #[serde(default)]
    pub units: Vec<String>,
    pub boot: Boot,
    #[serde(default)]
    pub platform: Option<super::Platform>,
    #[serde(default)]
    pub runner: Option<super::Runner>,
    #[serde(default)]
    pub sources: BTreeMap<String, Source>,
    #[serde(skip)]
    pub overrides: BTreeMap<String, std::path::PathBuf>,
}

impl Project {
    pub fn validate(&self, path: &Path) -> Result<(), Error> {
        for (name, override_path) in &self.overrides {
            if !self.sources.contains_key(name) || override_path.as_os_str().is_empty() {
                return Err(Error::Declaration {
                    path: path.parent().unwrap().join(".vanllia/overrides.toml"),
                    message: format!("unknown or empty source override {name}"),
                });
            }
        }
        if self.schema != 1 || self.boot.system.is_empty() {
            return Err(Error::Declaration {
                path: path.to_path_buf(),
                message: "expected schema = 1 and a nonempty boot.system".into(),
            });
        }
        if self.sources.keys().any(|key| key.is_empty()) || self.units.iter().any(String::is_empty)
        {
            return Err(Error::Declaration {
                path: path.to_path_buf(),
                message: "source names and Unit IDs must be nonempty".into(),
            });
        }
        for (name, source) in &self.sources {
            source.request().map_err(|message| Error::Declaration {
                path: path.to_path_buf(),
                message: format!("source `{name}`: {message}"),
            })?;
        }
        Ok(())
    }

    pub(crate) fn request(
        &self,
        name: &str,
        source: &Source,
    ) -> Result<crate::preparation::Request, String> {
        match self.overrides.get(name) {
            Some(path) => Ok(crate::preparation::Request::Local(path.clone())),
            None => source.request(),
        }
    }
}
