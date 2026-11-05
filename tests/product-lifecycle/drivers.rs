use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};
use unit::event::Event;

#[test]
#[ignore = "executes QEMU with IOMMU and a read-only VirtIO disk; requires exact artifact and firmware inputs"]
fn drivers_serve_and_reclaim_instances() {
    run().unwrap();
}

fn run() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let artifact =
        PathBuf::from(std::env::var_os("VANLLIA_TEST_ARTIFACT").ok_or("artifact input")?);
    let receipt = vanllia::runner::load(&artifact).map_err(|e| e.to_string())?;
    let firmware =
        PathBuf::from(std::env::var_os("VANLLIA_TEST_FIRMWARE").ok_or("firmware input")?);
    let hash = std::env::var("VANLLIA_TEST_FIRMWARE_SHA256").map_err(|e| e.to_string())?;
    let length: u64 = std::env::var("VANLLIA_TEST_FIRMWARE_BYTES")
        .map_err(|e| e.to_string())?
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let firmware = std::fs::read(firmware).map_err(|e| e.to_string())?;
    if length == 0
        || firmware.len() as u64 != length
        || format!("{:x}", Sha256::digest(&firmware)) != hash.to_ascii_lowercase()
    {
        return Err("firmware identity".into());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = root.join(".vanllia/runs").join(format!("drivers-{stamp}"));
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    for (name, input) in [
        ("kernel.elf", receipt.kernel.as_ref().ok_or("kernel input")?),
        ("initrd.img", &receipt.initrd),
    ] {
        let bytes =
            std::fs::read(receipt.directory.join(&input.path)).map_err(|e| e.to_string())?;
        if bytes.len() as u64 != input.bytes
            || format!("{:x}", Sha256::digest(&bytes)) != input.sha256
        {
            return Err("artifact changed before snapshot".into());
        }
        std::fs::write(directory.join(name), bytes).map_err(|e| e.to_string())?;
    }
    std::fs::write(directory.join("firmware.bin"), firmware).map_err(|e| e.to_string())?;
    let disk = vec![0u8; 4 * 1024 * 1024];
    let disk_hash = format!("{:x}", Sha256::digest(&disk));
    std::fs::write(directory.join("disk.raw"), disk).map_err(|e| e.to_string())?;
    let args = [
        "-machine",
        "virt,iommu-sys=on",
        "-m",
        "256M",
        "-smp",
        "1",
        "-bios",
        "firmware.bin",
        "-kernel",
        "kernel.elf",
        "-initrd",
        "initrd.img",
        "-drive",
        "file=disk.raw,if=none,id=driverprobe,format=raw,readonly=on",
        "-device",
        "virtio-blk-pci,drive=driverprobe,disable-legacy=on,iommu_platform=on",
        "-display",
        "none",
        "-monitor",
        "none",
        "-serial",
        "file:serial.log",
        "-no-reboot",
    ];
    let input = serde_json::json!({
        "artifact": receipt, "firmware_sha256": hash, "firmware_bytes": length,
        "disk_sha256": disk_hash, "disk_bytes": 4 * 1024 * 1024,
        "command": "qemu-system-riscv64", "args": args,
    });
    std::fs::write(
        directory.join("input.json"),
        serde_json::to_vec_pretty(&input).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    let mut command = Command::new("qemu-system-riscv64");
    command.current_dir(&directory).args(args);
    vanllia_build::workflow::process::initialize()?;
    let status =
        vanllia_build::workflow::process::observe(&mut command, &directory.join("process"), 60)?;
    let serial = std::fs::read(directory.join("serial.log")).map_err(|e| e.to_string())?;
    let verdict = verify(&serial);
    let passed = status.code() == Some(0) && verdict.is_ok();
    let report = serde_json::json!({
        "passed": passed, "vm_exit": status.code(), "error": verdict.err(),
        "serial_sha256": format!("{:x}", Sha256::digest(&serial)),
        "required_native": "all four drivers ready, discovery probe Reap/zero, all five instances retired, System complete",
    });
    std::fs::write(
        directory.join("verdict.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!("driver evidence: {}", directory.display());
    if passed {
        Ok(())
    } else {
        Err(format!("driver smoke failed: {report}"))
    }
}

fn verify(serial: &[u8]) -> Result<(), String> {
    vanllia::testing::verify(
        serial,
        "test.virtio-blk-discovery",
        vanllia::testing::Expectation::Clean,
    )
    .map_err(|e| e.to_string())?;
    let mut bound = BTreeMap::new();
    let mut ready = BTreeSet::new();
    for event in vanllia::testing::events(serial).map_err(|e| e.to_string())? {
        match event {
            Event::Bound { unit, task, .. } => {
                bound.insert(task, unit);
            }
            Event::Ready { task } => {
                ready.insert(task);
            }
            _ => {}
        }
    }
    let units: BTreeSet<_> = bound.values().map(String::as_str).collect();
    if bound.len() != 5
        || units
            != BTreeSet::from([
                "hub",
                "iommu",
                "pci",
                "virtio-blk",
                "test.virtio-blk-discovery",
            ])
    {
        return Err(format!("unexpected instance set: {units:?}"));
    }
    for name in ["hub", "iommu", "pci", "virtio-blk"] {
        let (task, _) = bound
            .iter()
            .find(|(_, unit)| unit.as_str() == name)
            .unwrap();
        if !ready.contains(task) {
            return Err(format!("missing native readiness: {name}"));
        }
    }
    Ok(())
}
