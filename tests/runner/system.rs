//! Cross-repository System/Twine acceptance belongs to the composition owner.
use kernel_verification::{process, *};
use std::{fs, path::Path, process::Command, time::Duration};
const SYSTEM: &str = "ee5584bb6b6dae24ebd93e8ecd822cdb9d4c8efd";
const TWINE: &str = "932454c07bd2673ac7d6d9612d36182aae71e8b2";
pub fn run(kernel: &Path, fixture: &Path, twine: &Path, root: &Path) -> Result<()> {
    require(revision(fixture)? == SYSTEM, "unreviewed System fixture")?;
    require(revision(twine)? == TWINE, "unreviewed Twine fixture")?;
    fresh(root)?;
    let root = root.canonicalize()?;
    let source = revision(kernel)?;
    archive(kernel, &source, &root.join("provider"))?;
    archive(fixture, SYSTEM, &root.join("fixture"))?;
    archive(twine, TWINE, &root.join("twine"))?;
    let qemu = std::env::var("QEMU_RISCV64").unwrap_or("qemu-system-riscv64".into());
    let firmware = Path::new(&std::env::var("QEMU_OPENSBI")?).canonicalize()?;
    let mut inputs = serde_json::json!({"source":source,"fixture":SYSTEM,"twine":TWINE,"tools":tools(&qemu,&firmware)?,"builds":[],"sha256":{}});
    for (name, subdir) in [
        ("guest", "tests/guest/runtime"),
        ("platform", "tests/guest/runtime/platform"),
    ] {
        let buildroot = root.join("fixture").join(subdir);
        fs::copy(
            buildroot.join("Cargo.lock"),
            root.join(format!("{name}.original.Cargo.lock")),
        )?;
        let mut command = Command::new("cargo");
        command
            .args(["build", "--release", "--target", TARGET, "--manifest-path"])
            .arg(buildroot.join("Cargo.toml"));
        patch_provider(&mut command, &root.join("provider"))?;
        if name == "guest" {
            command.arg("--config").arg(format!(
                "patch.\"https://github.com/shuyra-arche/twine.git\".twine.path={}",
                serde_json::to_string(&root.join("twine"))?
            ));
        }
        command
            .current_dir(root.join("provider"))
            .env("RUSTFLAGS", FLAGS);
        command
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_ENCODED_RUSTFLAGS");
        for phase in ["resolve", "locked"] {
            if phase == "locked" {
                command.arg("--locked");
            }
            inputs["builds"]
                .as_array_mut()
                .unwrap()
                .push(format!("{command:?}").into());
            json(&root.join("inputs.json"), &inputs)?;
            process::build(&mut command, &root.join(format!("{name}-{phase}")))?;
        }
        fs::copy(
            buildroot.join("Cargo.lock"),
            root.join(format!("{name}.Cargo.lock")),
        )?;
        fs::copy(
            buildroot
                .join("target")
                .join(TARGET)
                .join("release")
                .join(format!("system-runtime-{name}")),
            root.join(format!("{name}.elf")),
        )?;
    }
    fs::write(
        root.join("initrd.bin"),
        capsule::system_initrd(&fs::read(root.join("guest.elf"))?)?,
    )?;
    for name in [
        "guest.elf",
        "platform.elf",
        "initrd.bin",
        "guest.Cargo.lock",
        "platform.Cargo.lock",
    ] {
        inputs["sha256"][name] = digest(&root.join(name))?.into();
    }
    json(&root.join("inputs.json"), &inputs)?;
    let mut passed = true;
    for harts in [1, 4] {
        let destination = root.join(format!("harts-{harts}"));
        let mut command = Command::new(&qemu);
        command
            .args([
                "-accel",
                "tcg,thread=multi",
                "-machine",
                "virt",
                "-cpu",
                "rv64",
                "-smp",
            ])
            .arg(harts.to_string())
            .args([
                "-m",
                "128M",
                "-nographic",
                "-monitor",
                "none",
                "-nic",
                "none",
                "-bios",
            ])
            .arg(&firmware)
            .arg("-kernel")
            .arg(root.join("platform.elf"))
            .arg("-initrd")
            .arg(root.join("initrd.bin"));
        let mut result = process::run(&mut command, &destination, Duration::from_secs(120))?;
        let log = fs::read_to_string(destination.join("serial.log"))?;
        passed &= process::accept(
            &mut result,
            &log,
            &[
                "PASS system-runtime",
                "PASS supply-native",
                "PASS system-rpc",
                "PASS supply-window",
                "task: all tasks exited",
            ]
            .map(String::from),
        );
        json(&destination.join("result.json"), &result)?;
    }
    require(
        passed,
        format!("System acceptance failed: {}", root.display()),
    )
}
