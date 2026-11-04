//! Thin host adapter: parse product scene/profile/root and delegate to vanllia-build.

use std::path::{Path, PathBuf};

use unit::Deployment;
use vanllia_build::{ImageBuild, build_with, image_inventory, startup_order};

fn profile(value: &str) -> (&str, &'static str) {
    match value {
        "debug" => ("debug", "dev"),
        "release" => ("release", "release"),
        other => (other, Box::leak(other.to_string().into_boxed_str())),
    }
}

fn default_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}

struct Invocation {
    command: String,
    scene: String,
    profile: String,
    root: PathBuf,
}

fn parse() -> Result<Invocation, String> {
    let mut command = None;
    let mut scene = None;
    let mut profile = None;
    let mut root = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--root" {
            root = Some(args.next().ok_or("--root needs a path")?);
        } else if let Some(path) = arg.strip_prefix("--root=") {
            if path.is_empty() {
                return Err("--root needs a path".to_string());
            }
            root = Some(path.to_string());
        } else if arg.starts_with('-') {
            return Err(format!("unknown option `{arg}`"));
        } else if command.is_none() {
            command = Some(arg);
        } else if scene.is_none() {
            scene = Some(arg);
        } else if profile.is_none() {
            profile = Some(arg);
        } else {
            return Err(format!("extra argument `{arg}`"));
        }
    }
    let command = command.unwrap_or_else(|| "build".to_string());
    if command != "check" && command != "build" {
        return Err(format!("unknown command `{command}` (expected check or build)"));
    }
    let root = match root {
        Some(path) => {
            let path = PathBuf::from(path);
            path.canonicalize()
                .map_err(|error| format!("root {}: {error}", path.display()))?
        }
        None => default_root(),
    };
    Ok(Invocation {
        command,
        scene: scene.unwrap_or_else(|| "product".to_string()),
        profile: profile.unwrap_or_else(|| "release".to_string()),
        root,
    })
}

fn run(invocation: Invocation) -> Result<(), String> {
    let (label, cargo_profile) = profile(&invocation.profile);
    let Some(deployment) = Deployment::find(vanllia_composition::DEPLOYMENTS, &invocation.scene) else {
        let known = vanllia_composition::DEPLOYMENTS
            .iter()
            .map(|deployment| deployment.name)
            .collect::<Vec<_>>()
            .join(" / ");
        return Err(format!(
            "unknown scene {} (known: {known})",
            invocation.scene
        ));
    };
    let builds: Vec<ImageBuild> = vanllia_composition::IMAGE_BUILDS
        .iter()
        .map(|build| ImageBuild {
            profile: cargo_profile,
            ..*build
        })
        .collect();

    if invocation.command == "check" {
        let inventory = image_inventory(vanllia_composition::PROGRAMS, deployment, &builds)?;
        let order = startup_order(vanllia_composition::PROGRAMS, deployment)?;
        println!(
            "check: scene {} ({} images)",
            deployment.name,
            inventory.len()
        );
        for (image, kind) in &inventory {
            println!("image: {image} ({kind:?})");
        }
        println!("order: {}", order.join(" -> "));
        return Ok(());
    }

    let base = invocation.root.join("target/product").join(label);
    let work = base.join("work");
    let output = base.join("initrd.img");
    let at = build_with(
        &invocation.root,
        &work,
        &output,
        vanllia_composition::PROGRAMS,
        deployment,
        &builds,
    )?;
    println!("ok  {}", at.display());
    Ok(())
}

fn main() {
    let invocation = match parse() {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("vanllia-image: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) = run(invocation) {
        eprintln!("vanllia-image: {error}");
        std::process::exit(1);
    }
}
