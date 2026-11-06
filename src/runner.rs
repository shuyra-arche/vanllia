mod completed;
pub use completed::Completed;

use crate::{
    Error,
    build::{Artifact, Receipt},
    config::Runner,
    error,
};
use std::{
    path::{Component, Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
use vanllia_build::workflow::process;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Load and validate every recorded byte before accepting an existing artifact.
pub fn load(directory: &Path) -> Result<Receipt, Error> {
    let directory = directory
        .canonicalize()
        .map_err(|e| error::io(directory, e))?;
    let bytes =
        std::fs::read(directory.join("receipt.json")).map_err(|e| error::io(&directory, e))?;
    let mut receipt: Receipt =
        serde_json::from_slice(&bytes).map_err(|e| Error::Workflow(e.to_string()))?;
    if receipt.schema != 1 {
        return Err(Error::Workflow("unsupported artifact receipt".into()));
    }
    receipt.directory = directory;
    validate(&receipt)?;
    Ok(receipt)
}

pub fn validate(receipt: &Receipt) -> Result<(), Error> {
    for artifact in std::iter::once(&receipt.initrd)
        .chain(receipt.kernel.iter())
        .chain(receipt.firmware.iter())
        .chain(receipt.images.values())
        .chain(receipt.locks.values())
    {
        checked(receipt, artifact)?;
    }
    Ok(())
}

fn checked(receipt: &Receipt, artifact: &Artifact) -> Result<PathBuf, Error> {
    if artifact.path.as_os_str().is_empty()
        || artifact
            .path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(Error::Workflow(
            "artifact paths must stay inside their receipt directory".into(),
        ));
    }
    let path = receipt.directory.join(&artifact.path);
    let canonical = path.canonicalize().map_err(|e| error::io(&path, e))?;
    if !canonical.starts_with(&receipt.directory) {
        return Err(Error::Workflow(
            "artifact symlink escapes receipt directory".into(),
        ));
    }
    let bytes = std::fs::read(&canonical).map_err(|e| error::io(&canonical, e))?;
    if bytes.len() as u64 != artifact.bytes || crate::build::digest(&bytes) != artifact.sha256 {
        return Err(Error::Workflow(format!(
            "artifact changed: {}",
            path.display()
        )));
    }
    Ok(canonical)
}

/// Each execution owns a fresh directory and process group. The kernel and
/// initrd are snapshots of the exact validated artifact, never rebuilt here.
pub fn run(root: &Path, receipt: &Receipt, runner: &Runner) -> Result<Completed, Error> {
    if runner.cpus == 0
        || runner.timeout == 0
        || runner.machine.is_empty()
        || runner.memory.is_empty()
    {
        return Err(Error::Workflow(
            "runner machine/memory/cpus/timeout must be nonempty and nonzero".into(),
        ));
    }
    validate(receipt)?;
    let kernel = receipt
        .kernel
        .as_ref()
        .ok_or_else(|| Error::Workflow("run requires boot.kernel".into()))?;
    let firmware = receipt
        .firmware
        .as_ref()
        .ok_or_else(|| Error::Workflow("run requires a pinned boot.firmware input".into()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Workflow(e.to_string()))?
        .as_nanos();
    let directory = root.join(".vanllia/runs").join(format!(
        "{stamp}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).map_err(|e| error::io(&directory, e))?;
    for (name, artifact) in [
        ("kernel.elf", kernel),
        ("initrd.img", &receipt.initrd),
        ("firmware.bin", firmware),
    ] {
        let path = checked(receipt, artifact)?;
        let bytes = std::fs::read(&path).map_err(|e| error::io(&path, e))?;
        if crate::build::digest(&bytes) != artifact.sha256 {
            return Err(Error::Workflow(
                "artifact changed while snapshotting run inputs".into(),
            ));
        }
        std::fs::write(directory.join(name), bytes).map_err(|e| error::io(&directory, e))?;
    }
    std::fs::write(
        directory.join("input.json"),
        serde_json::to_vec_pretty(receipt).map_err(|e| Error::Workflow(e.to_string()))?,
    )
    .map_err(|e| error::io(&directory, e))?;
    let mut command = Command::new("qemu-system-riscv64");
    command
        .current_dir(&directory)
        .args(["-machine", &runner.machine, "-m", &runner.memory, "-smp"])
        .arg(runner.cpus.to_string())
        .args([
            "-bios",
            "firmware.bin",
            "-kernel",
            "kernel.elf",
            "-initrd",
            "initrd.img",
            "-display",
            "none",
            "-monitor",
            "none",
            "-serial",
            "file:serial.log",
            "-no-reboot",
        ]);
    let status = process::observe(&mut command, &directory.join("process"), runner.timeout)
        .map_err(Error::Workflow)?;
    if status.code().is_none() {
        return Err(Error::Workflow(
            "VM terminated without a native exit code".into(),
        ));
    }
    let serial =
        std::fs::read(directory.join("serial.log")).map_err(|e| error::io(&directory, e))?;
    Ok(Completed {
        directory,
        status: status.code().unwrap(),
        serial,
    })
}
