mod boot;
mod input;
mod kernel;
mod overrides;
mod project;
mod runner;
mod source;

pub use boot::Boot;
pub use input::Input;
pub use kernel::Kernel;
pub use overrides::Overrides;
pub use project::Project;
pub use runner::Runner;
pub use source::Source;

use crate::{Error, error};
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| error::io(path, e))?;
    toml_edit::de::from_str(&text).map_err(|e| Error::Declaration {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

pub fn project(root: &Path) -> Result<Project, Error> {
    let mut project: Project = read(&root.join("vanllia.toml"))?;
    project.overrides = overrides(root)?.sources;
    project.validate(&root.join("vanllia.toml"))?;
    Ok(project)
}

pub(crate) fn overrides(root: &Path) -> Result<Overrides, Error> {
    let path = root.join(".vanllia/overrides.toml");
    match std::fs::metadata(&path) {
        Ok(_) => {
            let overrides: Overrides = read(&path)?;
            if overrides.schema != 1 {
                return Err(Error::Declaration {
                    path,
                    message: "expected workspace override schema=1".into(),
                });
            }
            Ok(overrides)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Overrides::default()),
        Err(source) => Err(error::io(&path, source)),
    }
}
mod platform;
pub use platform::Platform;
