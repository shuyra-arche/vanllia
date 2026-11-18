use crate::{capsule, support::*};
use std::{fs, path::Path, process::Command, time::Instant};
use vanllia_build::workflow::process;

pub fn run(kernel: &Path, fixture: &Path, root: &Path, profile: &str, version: u8) -> Result<()> {
    fresh(root)?;
    let root = root.canonicalize()?;
    let source = revision(kernel)?;
    let pin = revision(fixture)?;
    archive(kernel, &source, &root.join("provider"))?;
    archive(fixture, &pin, &root.join("fixture"))?;
    let provider = root.join("provider");
    let composition = root.join("fixture/tests/memory");
    let qemu = std::env::var("QEMU_RISCV64").unwrap_or("qemu-system-riscv64".into());
    let firmware = Path::new(&std::env::var("QEMU_OPENSBI")?).canonicalize()?;
    let mut inputs = serde_json::json!({"source":source,"fixture":pin,"profile":profile,
        "capsule_version":version,"deadline_seconds":if profile == "debug" { 240 } else { 120 }, "tools":tools(&qemu,&firmware)?, "builds":[]});
    let mut build = Command::new("cargo");
    build
        .args(["build", "--manifest-path"])
        .arg(composition.join("Cargo.toml"))
        .args(["--target", TARGET]);
    if profile == "release" {
        build.arg("--release");
    }
    patch_provider(&mut build, &provider)?;
    build
        .current_dir(root.join("fixture"))
        .env("RUSTFLAGS", FLAGS)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    // Only provider substitution may change the selected dependency graph.
    // Retain the original and resolved locks, then build the exact resolved graph.
    fs::copy(
        composition.join("Cargo.lock"),
        root.join("original.Cargo.lock"),
    )?;
    inputs["builds"]
        .as_array_mut()
        .unwrap()
        .push(format!("{build:?}").into());
    json(&root.join("inputs.json"), &inputs)?;
    process::run(&mut build, &root.join("resolve"), 900)?;
    build.arg("--locked");
    inputs["builds"]
        .as_array_mut()
        .unwrap()
        .push(format!("{build:?}").into());
    json(&root.join("inputs.json"), &inputs)?;
    process::run(&mut build, &root.join("build"), 900)?;
    let binaries = composition.join("target").join(TARGET).join(profile);
    for (from, to) in [
        (binaries.join("memory-guest"), "guest.elf"),
        (binaries.join("memory-platform"), "platform.elf"),
        (composition.join("Cargo.lock"), "Cargo.lock"),
    ] {
        fs::copy(from, root.join(to))?;
    }
    let image = root.join("initrd.bin");
    fs::write(
        &image,
        capsule::initrd(&fs::read(root.join("guest.elf"))?, version)?,
    )?;
    let disk = root.join("disk.raw");
    fs::write(&disk, vec![0; 1024 * 1024])?;
    let mut hashes = serde_json::Map::new();
    for name in ["guest.elf", "platform.elf", "initrd.bin", "Cargo.lock"] {
        hashes.insert(name.into(), digest(&root.join(name))?.into());
    }
    inputs["sha256"] = hashes.into();
    json(&root.join("inputs.json"), &inputs)?;
    let mut command = Command::new(qemu);
    command.args(["-accel","tcg,thread=single","-machine","virt,iommu-sys=on","-cpu","rv64","-smp","1","-m","128M","-nographic","-monitor","none","-bios"])
        .arg(firmware).arg("-kernel").arg(root.join("platform.elf")).arg("-initrd").arg(image)
        .arg("-drive").arg(format!("file={},format=raw,if=none,id=disk,aio=threads",disk.display()))
        .args(["-device","virtio-blk-pci,drive=disk,disable-legacy=on,iommu_platform=on,ats=off,addr=1,ioeventfd=off,vectors=0"]);
    let boot = root.join("boot");
    let started = Instant::now();
    // The existing Vanllia Host executor enforces process-group cleanup and
    // preserves its own execution/result/exit evidence even on an error.
    let observed = process::observe(
        &mut command,
        &boot,
        if profile == "debug" { 240 } else { 120 },
    );
    // Preserve the historical serial.log contract for CI evidence and review.
    if boot.join("stdout.log").is_file() {
        fs::copy(boot.join("stdout.log"), boot.join("serial.log"))?;
    }
    let status = observed?;
    let serial = fs::read_to_string(boot.join("serial.log"))?;
    let passed = if version < 13 {
        status.code().is_some()
            && matches!(status.code(), Some(0 | 1))
            && [
                "shuyra: boot failed",
                "assemble boot capsule",
                "virtual address not in any declared map",
            ]
            .iter()
            .all(|marker| serial.contains(marker))
            && !serial.contains("START native-memory")
            && !serial.contains("FAIL")
            && !serial.contains("[kernel] fatal")
    } else {
        let mut markers: Vec<String> = [
            "PASS native-memory",
            "PASS native-version",
            "PASS native-fault",
            "PASS native-ipc",
            "PASS native-swap",
            "PASS native-virtqueue",
            "PASS acceptance-unit",
            "PASS acceptance-scopes",
            "PASS acceptance-replace",
            "PASS acceptance-aba",
            "PASS acceptance-lineage",
            "PASS acceptance-share",
            "PASS acceptance-foreign-control",
            "PASS acceptance-authority",
            "PASS acceptance-maintenance",
            "iommu: command engine closed",
            "task: all tasks exited",
        ]
        .map(String::from)
        .into();
        for mode in ["Revoke", "Return", "Release", "Delete"] {
            markers.push(format!("PASS acceptance-lifecycle mode={mode}"));
        }
        status.success()
            && !serial.contains("FAIL")
            && !serial.contains("[kernel] fatal")
            && markers.iter().all(|marker| serial.contains(marker))
    };
    let record = serde_json::json!({
        "command": format!("{command:?}"),
        "status": if status.code().is_some() { "exited" } else { "signal" },
        "code": status.code(),
        "elapsed_seconds": started.elapsed().as_secs_f64(),
        "passed": passed,
        "disk_sha256": digest(&disk)?
    });
    json(&boot.join("result.json"), &record)?;
    println!(
        "{} native {profile} v{version}: {}",
        if passed { "PASS" } else { "FAIL" },
        root.display()
    );
    require(
        passed,
        format!("native acceptance failed: {}", boot.display()),
    )
}
