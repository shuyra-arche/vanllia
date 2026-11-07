//! Real Kernel + manager + U-mode Driver. M-mode fixtures cannot satisfy this
//! contract. Test-only business code resides in guest/, never in Kernel.
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires Kernel built with qemu-edu-dma, QEMU 10.0+, and OpenSBI"]
fn protected_device_lifecycle_crosses_real_units() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guest_root = root.join("tests/device/guest");
    let kernel =
        PathBuf::from(std::env::var_os("SHUYRA_KERNEL_ELF").expect("explicit Kernel artifact"));
    let firmware =
        PathBuf::from(std::env::var_os("SHUYRA_FIRMWARE").expect("explicit OpenSBI artifact"));
    let qemu = std::env::var_os("QEMU_RISCV64").unwrap_or_else(|| "qemu-system-riscv64".into());
    let evidence = root.join("target/device-evidence");
    if evidence.exists() {
        std::fs::remove_dir_all(&evidence).unwrap();
    }
    std::fs::create_dir_all(&evidence).unwrap();
    let kernel_bytes = std::fs::read(&kernel).unwrap();
    let firmware_bytes = std::fs::read(&firmware).unwrap();
    std::fs::write(evidence.join("kernel.elf"), &kernel_bytes).unwrap();
    std::fs::write(evidence.join("firmware.bin"), &firmware_bytes).unwrap();
    let version = Command::new(&qemu).arg("--version").output().unwrap();
    assert!(version.status.success());
    for mode in ["clean", "crash", "readonly", "activation_exit"] {
        let mut build = Command::new("cargo");
        build
            .args([
                "+nightly",
                "build",
                "--locked",
                "--release",
                "--target",
                "riscv64gc-unknown-none-elf",
                "--manifest-path",
            ])
            .arg(guest_root.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", guest_root.join("target"));
        if mode != "clean" {
            build.args(["--features", mode]);
        }
        let built = build.output().unwrap();
        assert!(
            built.status.success(),
            "{}",
            String::from_utf8_lossy(&built.stderr)
        );
        let elf = std::fs::read(
            guest_root.join("target/riscv64gc-unknown-none-elf/release/device-guest"),
        )
        .unwrap();
        let capsule = vanllia_build::boot_capsule(&elf).unwrap();
        let mut initrd = vec![0; 4096];
        initrd[..4].copy_from_slice(&4096u32.to_le_bytes());
        initrd[4..8].copy_from_slice(&(capsule.len() as u32).to_le_bytes());
        initrd.extend(capsule);
        let dir = evidence.join(mode);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("initrd.img"), &initrd).unwrap();
        std::fs::write(dir.join("guest.elf"), &elf).unwrap();
        let mut command = Command::new(&qemu);
        command
            .args([
                "-M",
                "virt,iommu-sys=on",
                "-m",
                "128M",
                "-smp",
                "2",
                "-cpu",
                "rv64",
                "-bios",
            ])
            .arg(evidence.join("firmware.bin"))
            .arg("-kernel")
            .arg(evidence.join("kernel.elf"))
            .arg("-initrd")
            .arg(dir.join("initrd.img"))
            .args([
                "-device",
                "edu,addr=1.0,dma_mask=0xffffffffffffffff",
                "-nic",
                "none",
                "-nographic",
                "-monitor",
                "none",
            ]);
        let receipt = serde_json::json!({
            "command": format!("{command:?}"), "build": format!("{build:?}"),
            "qemu": String::from_utf8_lossy(&version.stdout),
            "kernel_sha256": format!("{:x}", Sha256::digest(&kernel_bytes)),
            "firmware_sha256": format!("{:x}", Sha256::digest(&firmware_bytes)),
            "guest_sha256": format!("{:x}", Sha256::digest(&elf)),
            "initrd_sha256": format!("{:x}", Sha256::digest(&initrd)),
            "guest_manifest_sha256": format!("{:x}", Sha256::digest(std::fs::read(guest_root.join("Cargo.toml")).unwrap())),
            "guest_lock_sha256": format!("{:x}", Sha256::digest(std::fs::read(guest_root.join("Cargo.lock")).unwrap())),
            "units": ["trusted test manager (S-mode)", "driver (U-mode)"],
            "platform_contract": "emulated QEMU EDU, no passthrough/hotplug/ATS"
        });
        std::fs::write(
            dir.join("receipt.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        // Files avoid a full pipe stalling QEMU before timeout detection.
        let serial = std::fs::File::create(dir.join("serial.log")).unwrap();
        let stderr = std::fs::File::create(dir.join("stderr.log")).unwrap();
        let mut child = command
            .stdout(Stdio::from(serial))
            .stderr(Stdio::from(stderr))
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(45);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("{mode}: QEMU timeout; evidence {}", dir.display());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let serial = std::fs::read_to_string(dir.join("serial.log")).unwrap();
        assert!(status.success(), "{mode}: {serial}");
        assert!(
            !serial.contains("FAIL") && !serial.contains("panicked"),
            "{mode}: {serial}"
        );
        if mode != "activation_exit" {
            let count: usize = serial
                .lines()
                .find_map(|line| line.strip_prefix("iommu-irqs: "))
                .and_then(|line| line.split_whitespace().find_map(|v| v.strip_prefix("fq=")))
                .unwrap()
                .parse()
                .unwrap();
            assert!(count > 0, "real FQ interrupt must reach Kernel");
            assert!(serial.contains("driver: hardware fault routed"));
        }
        let cause = if matches!(mode, "crash" | "activation_exit") {
            "fault"
        } else {
            "reap"
        };
        assert!(
            serial.contains(&format!(
                "PASS device-units ownership=1 borrow=1 driver-exit={cause} settled=1"
            )),
            "{mode}: {serial}"
        );
    }
    println!("device lifecycle evidence: {}", evidence.display());
}
