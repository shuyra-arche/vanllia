//! Test-only Git/source evidence around the existing Vanllia Host process runner.
//! These helpers do not express Kernel mechanisms or implement a second runner.
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, process::Command};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const TARGET: &str = "riscv64gc-unknown-none-elf";
pub const FLAGS: &str =
    "-Crelocation-model=static -Cforce-frame-pointers=yes -Ccode-model=medium -Dwarnings";

pub fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into().into())
    }
}

pub fn json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

pub fn digest(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(path)?;
    let mut bytes = [0; 65536];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hasher.update(&bytes[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn output(command: &mut Command) -> Result<String> {
    let observed = command.output()?;
    require(
        observed.status.success(),
        format!("{command:?}: {}", String::from_utf8_lossy(&observed.stderr)),
    )?;
    Ok(String::from_utf8(observed.stdout)?.trim().into())
}

pub fn git(repo: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg(format!("safe.directory={}", repo.display()));
    command.arg("-C").arg(repo);
    command
}

pub fn revision(repo: &Path) -> Result<String> {
    output(git(repo).args(["rev-parse", "HEAD"]))
}

pub fn archive(repo: &Path, revision: &str, destination: &Path) -> Result<()> {
    fs::create_dir(destination)?;
    let archive = destination.with_extension("tar");
    let status = git(repo)
        .args(["archive", "--format=tar", "--output"])
        .arg(&archive)
        .arg(revision)
        .status()?;
    require(status.success(), "git archive failed")?;
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(destination)
        .status()?;
    fs::remove_file(archive)?;
    require(status.success(), "tar extraction failed")
}

pub fn fresh(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(path)?;
    Ok(())
}

pub fn patch_provider(command: &mut Command, provider: &Path) -> Result<()> {
    for (package, folder) in [
        ("kernel", "server"),
        ("kernel-api", "api"),
        ("kernel-client", "client"),
    ] {
        command.arg("--config").arg(format!(
            "patch.\"https://github.com/shuyra-arche/kernel.git\".{package}.path={}",
            serde_json::to_string(&provider.join(folder))?
        ));
    }
    Ok(())
}

pub fn tools(qemu: &str, firmware: &Path) -> Result<serde_json::Value> {
    let version = output(Command::new(qemu).arg("--version"))?;
    require(
        version
            .lines()
            .next()
            .is_some_and(|line| line.contains("version 10.0.")),
        "only QEMU 10.0.x is reviewed",
    )?;
    Ok(serde_json::json!({
        "rustc": output(Command::new("rustc").arg("-Vv"))?,
        "cargo": output(Command::new("cargo").arg("-V"))?,
        "qemu": version,
        "firmware": firmware,
        "firmware_sha256": digest(firmware)?,
        "flags": FLAGS,
        "runner_sha256": digest(&std::env::current_exe()?)?,
        "cpu_max": fs::read_to_string("/sys/fs/cgroup/cpu.max").ok(),
        "host_status": fs::read_to_string("/proc/self/status").ok()
    }))
}
