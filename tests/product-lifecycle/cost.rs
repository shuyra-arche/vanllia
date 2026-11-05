#[path = "../syscall-cost/sample.rs"]
mod sample;

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use vanllia::build::{Artifact, Receipt};

fn input(name: &str) -> PathBuf {
    std::env::var_os(name)
        .unwrap_or_else(|| panic!("{name}"))
        .into()
}

fn snapshot(directory: &Path, path: &Path, name: &str, hash: &str, length: u64) -> Artifact {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), hash);
    assert_eq!(bytes.len() as u64, length);
    assert!(length > 0);
    std::fs::write(directory.join(name), bytes).unwrap();
    Artifact {
        path: name.into(),
        sha256: hash.into(),
        bytes: length,
    }
}

fn stage(original: &Receipt, directory: &Path) -> Receipt {
    std::fs::create_dir_all(directory).unwrap();
    for artifact in std::iter::once(&original.initrd)
        .chain(original.kernel.iter())
        .chain(original.firmware.iter())
        .chain(original.images.values())
        .chain(original.locks.values())
    {
        let destination = directory.join(&artifact.path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(original.directory.join(&artifact.path), destination).unwrap();
    }
    let mut staged = original.clone();
    staged.directory = directory.into();
    staged
}

#[test]
#[ignore = "executes ten QEMU boots; requires benchmark artifact and exact baseline/firmware identities"]
fn paired_syscall_cost() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let original = vanllia::runner::load(&input("VANLLIA_BENCH_ARTIFACT")).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = root.join(".vanllia/benchmarks").join(stamp.to_string());
    let mut current = stage(&original, &directory.join("current"));
    current.kernel = Some(snapshot(
        &current.directory,
        &input("VANLLIA_CURRENT_KERNEL"),
        "current.elf",
        &std::env::var("VANLLIA_CURRENT_SHA256").unwrap(),
        std::env::var("VANLLIA_CURRENT_BYTES")
            .unwrap()
            .parse()
            .unwrap(),
    ));
    current.firmware = Some(snapshot(
        &current.directory,
        &input("VANLLIA_TEST_FIRMWARE"),
        "firmware.bin",
        &std::env::var("VANLLIA_TEST_FIRMWARE_SHA256").unwrap(),
        std::env::var("VANLLIA_TEST_FIRMWARE_BYTES")
            .unwrap()
            .parse()
            .unwrap(),
    ));
    let mut baseline = stage(&current, &directory.join("baseline"));
    baseline.kernel = Some(snapshot(
        &baseline.directory,
        &input("VANLLIA_BASELINE_KERNEL"),
        "baseline.elf",
        &std::env::var("VANLLIA_BASELINE_SHA256").unwrap(),
        std::env::var("VANLLIA_BASELINE_BYTES")
            .unwrap()
            .parse()
            .unwrap(),
    ));
    for (name, receipt) in [("current", &current), ("baseline", &baseline)] {
        std::fs::write(
            directory.join(name).join("receipt.json"),
            serde_json::to_vec_pretty(receipt).unwrap(),
        )
        .unwrap();
    }
    let runner = vanllia::config::Runner {
        machine: "virt".into(),
        memory: "256M".into(),
        cpus: 1,
        timeout: 60,
    };
    vanllia_build::workflow::process::initialize().unwrap();
    let mut rows = Vec::new();
    for pair in 0..5 {
        let order = if pair % 2 == 0 {
            [("baseline", &baseline), ("current", &current)]
        } else {
            [("current", &current), ("baseline", &baseline)]
        };
        for (label, receipt) in order {
            let result = vanllia::runner::run(root, receipt, &runner).unwrap();
            let native = vanllia::testing::verify(
                &result.serial,
                "benchmark.syscall-cost",
                vanllia::testing::Expectation::Clean,
            );
            let metric = sample::Sample::decode(&result.serial);
            let identity = metric.as_ref().is_ok_and(|m| vanllia::testing::events(&result.serial).unwrap().iter().any(|e|
                matches!(e, unit::event::Event::Bound { unit, task, .. } if unit == "benchmark.syscall-cost" && *task == m.task)));
            let passed = result.status == 0 && native.is_ok() && metric.is_ok() && identity;
            let row = serde_json::json!({"pair":pair,"label":label,"passed":passed,"vm_exit":result.status,
                "native":format!("{native:?}"),"measurement":format!("{metric:?}"),"evidence":result.directory,
                "elapsed_ns":metric.as_ref().ok().map(|s| s.elapsed_ns),
                "iterations":metric.as_ref().ok().map(|s| s.iterations),
                "checksum":metric.as_ref().ok().map(|s| s.checksum),
                "initrd_sha256":receipt.initrd.sha256,"kernel_sha256":receipt.kernel.as_ref().unwrap().sha256,
                "firmware_sha256":receipt.firmware.as_ref().unwrap().sha256});
            std::fs::write(
                result.directory.join("verdict.json"),
                serde_json::to_vec_pretty(&row).unwrap(),
            )
            .unwrap();
            rows.push(row);
            std::fs::write(
                directory.join("samples.json"),
                serde_json::to_vec_pretty(&rows).unwrap(),
            )
            .unwrap();
            assert!(passed, "benchmark failed: {}", result.directory.display());
        }
    }
    println!("paired benchmark evidence: {}", directory.display());
}
