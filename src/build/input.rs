use super::{Artifact, Options};
use crate::{Error, config::Input, error};
use std::{path::Path, process::Command};

pub(super) fn prepare(
    root: &Path,
    work: &Path,
    spec: &Input,
    options: &Options,
) -> Result<Artifact, Error> {
    if spec.sha256.len() != 64
        || !spec.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || spec.bytes == 0
        || spec.path.is_some() == spec.url.is_some()
    {
        return Err(Error::Workflow(
            "firmware requires exactly one path/url and an exact SHA-256/length".into(),
        ));
    }
    let bytes = if let Some(path) = &spec.path {
        let path = root.join(path);
        std::fs::read(&path).map_err(|e| error::io(&path, e))?
    } else {
        let cache = root.join(".vanllia/inputs").join(&spec.sha256);
        if cache.is_file() {
            std::fs::read(&cache).map_err(|e| error::io(&cache, e))?
        } else {
            if options.offline {
                return Err(Error::SourceMissing(cache));
            }
            let destination = work.join("firmware.download");
            vanllia_build::workflow::process::capture(
                Command::new("curl")
                    .args([
                        "--fail",
                        "--location",
                        "--proto",
                        "=https",
                        "--max-time",
                        "120",
                        "--max-filesize",
                    ])
                    .arg(spec.bytes.to_string())
                    .arg("--output")
                    .arg(&destination)
                    .arg(spec.url.as_ref().unwrap()),
            )
            .map_err(Error::Workflow)?;
            let bytes = std::fs::read(&destination).map_err(|e| error::io(&destination, e))?;
            verify(&bytes, spec)?;
            std::fs::create_dir_all(cache.parent().unwrap()).map_err(|e| error::io(&cache, e))?;
            std::fs::rename(&destination, &cache).map_err(|e| error::io(&cache, e))?;
            bytes
        }
    };
    verify(&bytes, spec)?;
    super::write(work, "firmware.bin", &bytes)
}
fn verify(bytes: &[u8], spec: &Input) -> Result<(), Error> {
    if bytes.len() as u64 != spec.bytes || super::digest(bytes) != spec.sha256.to_ascii_lowercase()
    {
        return Err(Error::Workflow("firmware SHA-256/length mismatch".into()));
    }
    Ok(())
}
