//! Direct Kernel bootstrap probes for Toolchain's single-thread std backend.
//! This deliberately does not claim to exercise System's ELF loader/lifecycle.
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};
use vanllia_build::{boot_capsule, workflow::process};

fn input(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
        .canonicalize()
        .unwrap()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
#[ignore = "requires Toolchain std receipt, pinned Kernel/OpenSBI and QEMU; see tests/toolchain/README.md"]
fn std_main_allocator_and_termination() {
    let receipt_path = input("SHUYRA_TOOLCHAIN_RECEIPT");
    let receipt_bytes = fs::read(&receipt_path).unwrap();
    let receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes).unwrap();
    assert_eq!(receipt["schema"], 1);
    assert_eq!(receipt["target_build"], "passed");
    let kernel = input("SHUYRA_KERNEL_ELF");
    let firmware = input("SHUYRA_OPENSBI");
    let firmware_hash = digest(&fs::read(&firmware).unwrap());
    assert_eq!(
        firmware_hash,
        std::env::var("SHUYRA_FIRMWARE_SHA256").expect("pin firmware SHA-256")
    );
    let kernel_revision = std::env::var("SHUYRA_KERNEL_REVISION")
        .expect("record the Kernel source revision used to build the ELF");
    assert!(kernel_revision.len() == 40 && kernel_revision.bytes().all(|b| b.is_ascii_hexdigit()));
    let qemu = std::env::var_os("SHUYRA_QEMU").unwrap_or_else(|| "qemu-system-riscv64".into());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let evidence = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/toolchain")
        .join(format!("{stamp}-{}", std::process::id()));
    fs::create_dir_all(&evidence).unwrap();
    eprintln!("Toolchain bootstrap evidence: {}", evidence.display());
    let version = Command::new(&qemu).arg("--version").output().unwrap();
    assert!(version.status.success());
    let mut results = Vec::new();
    for (name, reason, note) in [
        ("toolchain-std", "0x0", "toolchain: std main returned"),
        ("std-exit", "0x17", "toolchain: std process exit"),
        ("std-panic", "0xffffff01", "toolchain: std abort"),
        ("std-oom", "0xffffff01", "toolchain: std abort"),
    ] {
        let artifact = &receipt["artifacts"][name];
        let elf = fs::read(artifact["path"].as_str().expect("artifact path")).unwrap();
        let hash = digest(&elf);
        assert_eq!(
            hash,
            artifact["elf"]["sha256"].as_str().expect("artifact hash"),
            "stale Toolchain artifact: {name}"
        );
        let capsule = boot_capsule(&elf).expect("encode the actual Toolchain ELF");
        let mut initrd = vec![0; 4096];
        initrd[..4].copy_from_slice(&4096u32.to_le_bytes());
        initrd[4..8].copy_from_slice(&u32::try_from(capsule.len()).unwrap().to_le_bytes());
        initrd.extend_from_slice(&capsule);
        let directory = evidence.join(name);
        fs::create_dir_all(&directory).unwrap();
        let initrd_path = directory.join("initrd.bin");
        fs::write(&initrd_path, &initrd).unwrap();
        let result = process::run(
            Command::new(&qemu)
                .args(["-machine", "virt", "-m", "256M", "-smp", "1", "-nographic"])
                .arg("-bios")
                .arg(&firmware)
                .arg("-kernel")
                .arg(&kernel)
                .arg("-initrd")
                .arg(&initrd_path),
            &directory,
            20,
        );
        let uart = fs::read_to_string(directory.join("stdout.log"))
            .unwrap_or_default()
            .replace('\r', "");
        let expected = format!("exit tid=1 reason={reason} note: {note}");
        let passed = result.is_ok()
            && uart.lines().filter(|line| *line == expected).count() == 1
            && uart
                .lines()
                .any(|line| line == "task: all tasks exited, system halted")
            && !uart.contains("boot failed");
        results.push(serde_json::json!({"fixture": name, "elf_sha256": hash,
            "initrd_sha256": digest(&initrd), "expected_exit": expected, "passed": passed}));
        fs::write(evidence.join("result.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "scope": "direct Kernel bootstrap; System loader not exercised",
            "toolchain_receipt_sha256": digest(&receipt_bytes), "toolchain_receipt": receipt,
            "kernel_revision": kernel_revision, "kernel_sha256": digest(&fs::read(&kernel).unwrap()),
            "firmware_sha256": firmware_hash, "qemu": String::from_utf8_lossy(&version.stdout),
            "scenarios": results,
        })).unwrap()).unwrap();
        assert!(
            passed,
            "{name}: {result:?}; expected {expected:?}; evidence: {}\n{uart}",
            directory.display()
        );
    }
}
