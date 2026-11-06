use super::{Artifact, Options, package};
use crate::{Error, config::Platform, error, image::Image, overlay};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct Compiler<'a> {
    root: &'a Path,
    work: &'a Path,
    platform: &'a Platform,
    patches: &'a [overlay::Patch],
    options: &'a Options,
    pub resolutions: BTreeMap<PathBuf, overlay::Resolution>,
}
impl<'a> Compiler<'a> {
    pub fn new(
        root: &'a Path,
        work: &'a Path,
        platform: &'a Platform,
        patches: &'a [overlay::Patch],
        options: &'a Options,
    ) -> Self {
        Self {
            root,
            work,
            platform,
            patches,
            options,
            resolutions: BTreeMap::new(),
        }
    }
    pub fn compile(&mut self, image: &Image, extra_features: &[String]) -> Result<Artifact, Error> {
        if !self.resolutions.contains_key(&image.workspace) {
            self.resolutions.insert(
                image.workspace.clone(),
                overlay::resolve(
                    self.root,
                    &image.manifest,
                    self.patches,
                    self.options.offline,
                )?,
            );
        }
        let resolution = &self.resolutions[&image.workspace];
        let settings = package::read(image)?;
        let key = super::digest(image.workspace.to_string_lossy().as_bytes());
        let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
        command
            .current_dir(&resolution.workspace)
            .args([
                "build",
                "--locked",
                "--message-format=json-render-diagnostics",
                "--manifest-path",
            ])
            .arg(&image.manifest)
            .arg("--config")
            .arg(&resolution.config)
            .arg("--target-dir")
            .arg(self.root.join(".vanllia/target").join(key))
            .args([
                "--target",
                &self.platform.target,
                "--profile",
                &self.platform.profile,
                "--package",
                &image.package,
                "--bin",
                &image.name,
            ]);
        if self.options.offline {
            command.arg("--offline");
        }
        if !settings.default_features {
            command.arg("--no-default-features");
        }
        let features = settings
            .features
            .iter()
            .chain(extra_features)
            .cloned()
            .collect::<BTreeSet<_>>();
        if !features.is_empty() {
            command
                .arg("--features")
                .arg(features.into_iter().collect::<Vec<_>>().join(","));
        }
        let output =
            vanllia_build::workflow::process::capture(&mut command).map_err(Error::Workflow)?;
        let mut executable = None;
        for line in output
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let value: serde_json::Value = serde_json::from_slice(line)
                .map_err(|e| Error::Workflow(format!("Cargo artifact JSON: {e}")))?;
            if value["reason"] == "compiler-artifact"
                && value["package_id"] == image.package_id
                && value["target"]["name"] == image.name
                && value["target"]["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
                && let Some(path) = value["executable"].as_str()
                && executable.replace(PathBuf::from(path)).is_some()
            {
                return Err(Error::Workflow(format!(
                    "ambiguous Cargo executable for {}",
                    image.name
                )));
            }
        }
        let path = executable.ok_or_else(|| {
            Error::Workflow(format!("Cargo produced no executable for {}", image.name))
        })?;
        let bytes = std::fs::read(&path).map_err(|e| error::io(&path, e))?;
        super::write(
            self.work,
            &format!("image-{}", super::digest(&bytes)),
            &bytes,
        )
    }
}
