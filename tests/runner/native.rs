use kernel_verification::{process, *};
use std::{fs, path::Path, process::Command, time::Duration};
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
        "capsule_version":version,"deadline_seconds":if profile == "debug" { 240 } else { 120 },"tools":tools(&qemu,&firmware)?, "builds":[]});
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
        .env("RUSTFLAGS", FLAGS);
    build
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_ENCODED_RUSTFLAGS");
    // Only provider substitutions may change resolution. Keep the old and
    // resolved graph, then insist the exact resolved graph builds with --locked.
    fs::copy(
        composition.join("Cargo.lock"),
        root.join("original.Cargo.lock"),
    )?;
    inputs["builds"]
        .as_array_mut()
        .unwrap()
        .push(format!("{build:?}").into());
    json(&root.join("inputs.json"), &inputs)?;
    process::build(&mut build, &root.join("resolve"))?;
    build.arg("--locked");
    inputs["builds"]
        .as_array_mut()
        .unwrap()
        .push(format!("{build:?}").into());
    json(&root.join("inputs.json"), &inputs)?;
    process::build(&mut build, &root.join("build"))?;
    let binaries = composition.join("target").join(TARGET).join(profile);
    for (from, to) in [
        (binaries.join("memory-guest"), "guest.elf"),
        (binaries.join("memory-platform"), "platform.elf"),
        (composition.join("Cargo.lock"), "Cargo.lock"),
    ] {
        fs::copy(from, root.join(to))?;
    }
    let image = root.join("initrd.bin");
    // Before removing the old Kernel test helper, prove the production
    // Vanllia encoder preserves the exact boot bytes for this real guest,
    // including the ABI 10/12 rejection cases.
    let guest = fs::read(root.join("guest.elf"))?;
    let previous = kernel_verification::capsule::initrd(&guest, version)?;
    let current = crate::capsule::initrd(&guest, version)?;
    require(previous == current, "native Capsule byte-for-byte parity failed")?;
    fs::write(&image, current)?;
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
    let mut result = process::run(
        &mut command,
        &boot,
        Duration::from_secs(if profile == "debug" { 240 } else { 120 }),
    )?;
    let serial = fs::read_to_string(boot.join("serial.log"))?;
    if version < 13 {
        result.passed = result.exited()
            && matches!(result.code, Some(0 | 1))
            && [
                "shuyra: boot failed",
                "assemble boot capsule",
                "virtual address not in any declared map",
            ]
            .iter()
            .all(|v| serial.contains(v))
            && !serial.contains("START native-memory")
            && !serial.contains("FAIL")
            && !serial.contains("[kernel] fatal");
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
        process::accept(&mut result, &serial, &markers);
    }
    let mut record = serde_json::to_value(&result)?;
    record["disk_sha256"] = digest(&disk)?.into();
    json(&boot.join("result.json"), &record)?;
    println!(
        "{} native {profile} v{version}: {}",
        if result.passed { "PASS" } else { "FAIL" },
        root.display()
    );
    require(
        result.passed,
        format!("native acceptance failed: {}", boot.display()),
    )
}
