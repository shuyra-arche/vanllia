use crate::{Error, image::Image, metadata};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Package {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default = "enabled")]
    pub default_features: bool,
    #[serde(default)]
    pub resources: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub runtime_resources: BTreeSet<String>,
}
fn enabled() -> bool {
    true
}

pub(super) fn read(image: &Image) -> Result<Package, Error> {
    let metadata = metadata::observe(&image.manifest)?;
    let package = metadata
        .packages
        .iter()
        .find(|package| package.id == image.package_id)
        .ok_or_else(|| Error::Workflow(format!("Cargo package {} disappeared", image.package)))?;
    match package.metadata.get("vanllia") {
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| Error::Declaration {
            path: image.manifest.clone(),
            message: e.to_string(),
        }),
        None => Ok(Package {
            default_features: true,
            ..Package::default()
        }),
    }
}
