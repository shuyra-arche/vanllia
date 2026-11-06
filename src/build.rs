mod artifact;
mod compiler;
mod input;
mod options;
mod package;
mod receipt;
mod snapshot;

pub use artifact::Artifact;
pub use options::Options;
pub use receipt::Receipt;

use crate::{
    Catalog, Deployment, Error, Selection, config, discovery, error, overlay, preparation,
};
use initrd::ledger::bundle::{self, File, Kind, Unit};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use vanllia_build::workflow::process;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Prepare sources, resolve Cargo, compile the selected images, then publish a
/// complete immutable artifact directory. A failed build publishes no receipt.
pub fn build(start: &Path, selection: &Selection, options: &Options) -> Result<Receipt, Error> {
    let root = discovery::find_root(start)?;
    let project = config::project(&root)?;
    project.validate(&root.join("vanllia.toml"))?;
    let _lease = vanllia_build::lease(&root.join(".vanllia/project")).map_err(Error::Workflow)?;
    if options.offline {
        preparation::locate(&root, &project)?;
    } else {
        preparation::prepare_locked(&root, &project, preparation::Mode::Locked)?;
    }
    let files = discovery::discover(&root, &project)?;
    let before = snapshot::read(&files.roots)?;
    let catalog = crate::load(&root)?;
    let deployment = crate::plan(&catalog, selection)?;
    let patches = overlay::discover(&files)?;
    let platform = project.platform.as_ref().ok_or_else(|| {
        Error::Workflow("build requires [platform] target in vanllia.toml".into())
    })?;
    if platform.target.is_empty() || platform.profile.is_empty() {
        return Err(Error::Workflow(
            "platform target/profile must be nonempty".into(),
        ));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Workflow(e.to_string()))?
        .as_nanos();
    let work = root.join(".vanllia/builds").join(format!(
        "{stamp}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&work).map_err(|e| error::io(&work, e))?;
    let result = assemble(&root, &work, &catalog, &deployment, &patches, options);
    let mut receipt = match result {
        Ok(receipt) if before == snapshot::read(&files.roots)? => receipt,
        Ok(_) => {
            return Err(Error::Workflow(format!(
                "source files changed during build; unpublished work: {}",
                work.display()
            )));
        }
        Err(why) => {
            std::fs::write(work.join("failure.txt"), why.to_string())
                .map_err(|e| error::io(&work, e))?;
            return Err(why);
        }
    };
    receipt.source_files = before;
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(|e| Error::Workflow(e.to_string()))?;
    std::fs::write(work.join("receipt.json"), bytes).map_err(|e| error::io(&work, e))?;
    // Directory rename publishes the initrd, kernel, locks and receipt together.
    let ready = root.join(".vanllia/artifacts");
    std::fs::create_dir_all(&ready).map_err(|e| error::io(&ready, e))?;
    let destination = ready.join(work.file_name().unwrap());
    std::fs::rename(&work, &destination).map_err(|e| error::io(&destination, e))?;
    Ok(Receipt {
        directory: destination,
        ..receipt
    })
}

fn assemble(
    root: &Path,
    work: &Path,
    catalog: &Catalog,
    deployment: &Deployment,
    patches: &[overlay::Patch],
    options: &Options,
) -> Result<Receipt, Error> {
    let platform = catalog.project().platform.as_ref().unwrap();
    let mut images = BTreeMap::new();
    let mut raw = BTreeSet::new();
    let mut records = Vec::new();
    for id in &deployment.order {
        let unit = &catalog.units()[id].unit;
        records.push((
            id.as_str(),
            unit_contract::record::encode(unit)
                .map_err(|e| Error::Workflow(format!("Unit {id}: {e}")))?,
        ));
        let kind = match unit.loader.space {
            unit_contract::document::Space::User => Kind::User,
            unit_contract::document::Space::Supervisor => Kind::Supervisor,
        };
        if images
            .insert(unit.loader.image.clone(), kind)
            .is_some_and(|previous| previous != kind)
        {
            return Err(Error::Workflow(format!(
                "conflicting execution spaces for image {}",
                unit.loader.image
            )));
        }
        for binding in &unit.supply.bindings {
            if binding.source == "system/initrd" {
                raw.insert(binding.resource.clone());
            }
        }
        for policy in &unit.control.construction {
            if let Some(id) = &policy.unit
                && !deployment.reasons.contains_key(id)
            {
                return Err(Error::Workflow(format!(
                    "Unit {} constructs {id}; add it to requires",
                    unit.id
                )));
            }
        }
    }
    let mut resource_paths = BTreeMap::new();
    for image in catalog.images().values() {
        let config = package::read(image)?;
        for (name, path) in config.resources {
            let path = image
                .manifest
                .parent()
                .unwrap()
                .join(path)
                .canonicalize()
                .map_err(|e| error::io(&image.manifest, e))?;
            if let Some(previous) = resource_paths.insert(name.clone(), path.clone())
                && previous != path
            {
                return Err(Error::Duplicate {
                    kind: "resource",
                    id: name,
                    first: previous,
                    second: path,
                });
            }
        }
    }
    let system_image = &catalog.units()[&deployment.system].unit.loader.image;
    let runtime_resources = package::read(&catalog.images()[system_image])?.runtime_resources;
    if runtime_resources.iter().any(|name| {
        name.is_empty() || catalog.images().contains_key(name) || resource_paths.contains_key(name)
    }) {
        return Err(Error::Workflow(
            "System runtime resource conflicts with a packaged file".into(),
        ));
    }
    for name in &raw {
        if catalog.images().contains_key(name) {
            images.entry(name.clone()).or_insert(Kind::User);
        }
    }
    let mut compiler = compiler::Compiler::new(root, work, platform, patches, options);
    let mut evidence = BTreeMap::new();
    let mut entries = Vec::new();
    for (name, kind) in &images {
        let image = catalog
            .images()
            .get(name)
            .ok_or_else(|| Error::UnknownImage {
                unit: "initrd".into(),
                image: name.clone(),
            })?;
        let artifact = compiler.compile(image, &[])?;
        let path = work.join(&artifact.path);
        let bytes = vanllia_build::runtime_image(&path, work).map_err(Error::Workflow)?;
        entries.push((*kind, name.clone(), bytes));
        evidence.insert(name.clone(), artifact);
    }
    for name in raw {
        if images.contains_key(&name) || runtime_resources.contains(&name) {
            continue;
        }
        let path = resource_paths
            .get(&name)
            .ok_or_else(|| Error::UnknownResource {
                resource: name.clone(),
            })?;
        entries.push((
            Kind::Resource,
            name,
            std::fs::read(path).map_err(|e| error::io(path, e))?,
        ));
    }
    let boot_image = entries
        .iter()
        .position(|(_, name, _)| name == system_image)
        .unwrap();
    let capsule = vanllia_build::boot_capsule(&entries[boot_image].2).map_err(Error::Workflow)?;
    let units: Vec<_> = records
        .iter()
        .map(|(id, record)| Unit { id, record })
        .collect();
    let files: Vec<_> = entries
        .iter()
        .map(|(kind, name, bytes)| File {
            kind: *kind,
            name,
            bytes,
        })
        .collect();
    let boot_unit = records
        .iter()
        .position(|(id, _)| *id == deployment.system)
        .unwrap();
    let initrd = bundle::pack(&units, &files, boot_unit, boot_image, &capsule)
        .map_err(|e| Error::Workflow(e.to_string()))?;
    let initrd = write(work, "initrd.img", &initrd)?;
    let kernel = catalog
        .project()
        .boot
        .kernel
        .as_ref()
        .map(|kernel| {
            let name = &kernel.image;
            let image = catalog
                .images()
                .get(name)
                .ok_or_else(|| Error::UnknownImage {
                    unit: "boot.kernel".into(),
                    image: name.clone(),
                })?;
            compiler.compile(image, &kernel.features)
        })
        .transpose()?;
    let firmware = catalog
        .project()
        .boot
        .firmware
        .as_ref()
        .map(|spec| input::prepare(root, work, spec, options))
        .transpose()?;
    let mut locks = BTreeMap::new();
    for (index, resolution) in compiler.resolutions.values().enumerate() {
        let bytes = std::fs::read(&resolution.lock).map_err(|e| error::io(&resolution.lock, e))?;
        locks.insert(
            resolution.workspace.clone(),
            write(work, &format!("cargo-{index}.lock"), &bytes)?,
        );
    }
    if root.join("vanllia.lock").is_file() {
        let bytes = std::fs::read(root.join("vanllia.lock")).map_err(|e| error::io(root, e))?;
        write(work, "vanllia.lock", &bytes)?;
    }
    let rustc = process::capture(Command::new("rustc").arg("-Vv")).map_err(Error::Workflow)?;
    let cargo = process::capture(
        Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())).arg("-V"),
    )
    .map_err(Error::Workflow)?;
    Ok(Receipt {
        schema: 1,
        platform: platform.clone(),
        kernel_features: catalog
            .project()
            .boot
            .kernel
            .as_ref()
            .map(|kernel| kernel.features.clone())
            .unwrap_or_default(),
        builder_sha256: {
            let path = std::env::current_exe().map_err(|e| error::io(work, e))?;
            digest(&std::fs::read(&path).map_err(|e| error::io(&path, e))?)
        },
        directory: PathBuf::new(),
        initrd,
        kernel,
        firmware,
        images: evidence,
        locks,
        source_files: BTreeMap::new(),
        rustc: String::from_utf8_lossy(&rustc).into(),
        cargo: String::from_utf8_lossy(&cargo).into(),
    })
}

fn write(directory: &Path, name: &str, bytes: &[u8]) -> Result<Artifact, Error> {
    let path = directory.join(name);
    std::fs::write(&path, bytes).map_err(|e| error::io(&path, e))?;
    Ok(Artifact {
        path: PathBuf::from(name),
        sha256: digest(bytes),
        bytes: bytes.len() as u64,
    })
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
