//! Managed development contexts. Products supply their own sources, declarations
//! host, workspaces, deployments and existing test runners in a JSON manifest.
mod config;
mod environment;
pub mod process;
mod source;

pub use config::{
    Cluster, Config, Deployment, Firmware, Host, Kernel, Package, Source, Test, Unit, Workspace,
};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{ImageBuild, executables, publication};
use process::{capture, run};

static CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn cancel() {
    CANCELLED.store(true, Ordering::SeqCst);
}
fn cancelled() -> bool {
    CANCELLED.load(Ordering::SeqCst)
}
pub(crate) fn active() -> Result<(), String> {
    if cancelled() {
        Err("execution cancelled before publication".into())
    } else {
        Ok(())
    }
}

fn cargo(config: &Config) -> Command {
    let mut command = Command::new("cargo");
    command.env("RUSTUP_TOOLCHAIN", &config.toolchain);
    command
}

fn json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    publication::write(
        path,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
}

fn load_overrides(state: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    match std::fs::read(state.join("overrides.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn literal(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn coordinates(config: &Config) -> ImageBuild {
    let kernel = &config.kernel;
    ImageBuild {
        image: "kernel",
        manifest: literal(kernel.manifest.clone()),
        package: literal(kernel.package.clone()),
        bin: literal(kernel.bin.clone()),
        target: literal(config.target.clone()),
        profile: literal(kernel.profile.clone()),
        features: Box::leak(
            kernel
                .features
                .iter()
                .cloned()
                .map(literal)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        default_features: true,
    }
}

fn host(
    config: &Config,
    context: &Path,
    evidence: &Path,
    command: &str,
    extra: &[String],
) -> Result<(), String> {
    let root = context.join("sources").join(&config.host.source);
    let mut cargo = cargo(config);
    cargo
        .args([
            "run",
            "--locked",
            "--target",
            &config.host_target,
            "--manifest-path",
        ])
        .arg(root.join(&config.host.manifest))
        .arg("--target-dir")
        .arg(context.join("host-target"))
        .args([
            "-p",
            &config.host.package,
            "--bin",
            &config.host.bin,
            "--",
            command,
            "--root",
        ])
        .arg(root)
        .args(extra)
        .current_dir(context);
    run(&mut cargo, evidence, 600)
}

fn prepare(config: &Config, base: &Path, state: &Path) -> Result<PathBuf, String> {
    match prepare_context(config, base, state) {
        Ok(context) => Ok(context),
        Err(error) => {
            if error.contains("stop_confirmed=false") {
                publication::write(&state.join("STOP_UNCONFIRMED"), error.as_bytes())
                    .map_err(|cleanup| format!("{error}; reservation failed: {cleanup}"))?;
            }
            Err(error)
        }
    }
}

fn prepare_context(config: &Config, base: &Path, state: &Path) -> Result<PathBuf, String> {
    let overrides = load_overrides(state)?;
    for name in overrides.keys() {
        if !config.sources.iter().any(|source| &source.name == name) {
            return Err(format!(
                "override {name} is not declared in this configuration"
            ));
        }
    }
    let mut identities = BTreeMap::new();
    for (name, path) in &overrides {
        identities.insert(name.clone(), source::snapshot(path, None)?);
    }
    let library = base
        .join(&config.library)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let library_files = source::snapshot(&library, None)?;
    let mut patch_hashes = Vec::new();
    let mut patch_inputs = Vec::new();
    for item in &config.sources {
        for patch in &item.patches {
            let bytes = std::fs::read(base.join(patch)).map_err(|e| e.to_string())?;
            patch_hashes.push(publication::digest(&bytes));
            patch_inputs.push(bytes);
        }
    }
    let build_environment = environment::selected(std::env::vars_os())?;
    let identity = serde_json::json!({"config": config, "overrides": overrides,
        "build_environment": build_environment,
        "source_files": identities, "library_files": library_files, "patches": patch_hashes});
    let key = publication::digest(&serde_json::to_vec(&identity).unwrap());
    let prepared = state.join("prepared").join(format!("{key}.json"));
    if prepared.exists() {
        let context: PathBuf =
            serde_json::from_slice(&std::fs::read(&prepared).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let expected: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.join("inputs.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let actual = serde_json::json!({"sources": source::inventory(&context.join("sources"))?,
            "library": source::inventory(&context.join("library"))?});
        if expected != actual {
            return Err(format!(
                "prepared inputs changed; context quarantined: {}",
                context.display()
            ));
        }
        return Ok(context);
    }
    // Failed preparation remains independent evidence; it is never reused.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let context = state.join("contexts").join(format!("{key}-{stamp}"));
    std::fs::create_dir_all(&context).map_err(|e| e.to_string())?;
    json(&context.join("identity.json"), &identity)?;
    let result = (|| {
        source::prepare(config, base, &context, &overrides, &patch_inputs)?;
        let sources: serde_json::Value = serde_json::from_slice(
            &std::fs::read(context.join("sources.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        for source in sources.as_array().ok_or("source evidence missing")? {
            if let Some(name) = source["name"].as_str()
                && overrides.contains_key(name)
                && identity["source_files"][name] != source["snapshot"]
            {
                return Err(format!("local checkout {name} changed during snapshot"));
            }
        }
        source::configure(config, base, &context)?;
        if source::inventory(&context.join("library"))? != identity["library_files"] {
            return Err("library changed during snapshot".into());
        }
        let mut rustc = Command::new("rustc");
        rustc.env("RUSTUP_TOOLCHAIN", &config.toolchain).arg("-Vv");
        publication::write(&context.join("rustc.txt"), &capture(&mut rustc)?)?;
        publication::write(
            &context.join("cargo.txt"),
            &capture(cargo(config).arg("--version"))?,
        )?;
        json(
            &context.join("inputs.json"),
            &serde_json::json!({"sources": source::inventory(&context.join("sources"))?,
            "library": source::inventory(&context.join("library"))?}),
        )?;
        active()?;
        json(
            &context.join("ready.json"),
            &serde_json::json!({"identity": identity, "context": context}),
        )?;
        json(&prepared, &context)
    })();
    if let Err(error) = &result {
        json(
            &context.join("failure.json"),
            &serde_json::json!({"error": error}),
        )
        .map_err(|record| format!("{error}; preparation failure record failed: {record}"))?;
    }
    result?;
    Ok(context)
}

fn firmware(config: &Config, out: &Path) -> Result<PathBuf, String> {
    let path = out.join("firmware.bin");
    capture(
        Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--proto",
                "=https",
                "--max-time",
                "120",
                "--output",
            ])
            .arg(&path)
            .arg(&config.firmware.url),
    )?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.len() as u64 != config.firmware.bytes
        || publication::digest(&bytes) != config.firmware.sha256
    {
        return Err("firmware digest/length mismatch; refusing input".into());
    }
    Ok(path)
}

fn build(config: &Config, context: &Path, out: &Path, name: &str) -> Result<(), String> {
    let deployment = config
        .deployments
        .get(name)
        .ok_or_else(|| format!("undeclared deployment {name}"))?;
    let initrd = out.join("initrd.img");
    let mut extra = deployment.host_args.clone();
    extra.extend([
        "--work".into(),
        context.join("image-work").to_string_lossy().into_owned(),
        "--out".into(),
        initrd.to_string_lossy().into_owned(),
    ]);
    host(config, context, &out.join("host"), "build", &extra)?;
    publication::write(
        &out.join("image-build.json"),
        &std::fs::read(context.join("image-work/build.json")).map_err(|e| e.to_string())?,
    )?;
    let kernel_root = context.join("sources").join(&config.kernel.source);
    let artifacts = executables(
        &kernel_root,
        &context.join("kernel-work"),
        &[coordinates(config)],
    )?;
    publication::write(
        &out.join("kernel.elf"),
        &std::fs::read(&artifacts[0].1).map_err(|e| e.to_string())?,
    )?;
    firmware(config, out)?;
    let mut inputs = BTreeMap::new();
    for file in ["initrd.img", "kernel.elf", "firmware.bin"] {
        let bytes = std::fs::read(out.join(file)).map_err(|e| e.to_string())?;
        inputs.insert(
            file,
            serde_json::json!({"sha256": publication::digest(&bytes), "bytes": bytes.len()}),
        );
    }
    json(
        &out.join("artifacts.json"),
        &serde_json::json!({"context": context, "deployment": name, "inputs": inputs}),
    )
}

fn expand(
    args: &[String],
    context: &Path,
    out: &Path,
    harts: u32,
    executable: Option<&Path>,
) -> Result<Command, String> {
    let variables = [
        ("{tool}", context.join("library")),
        ("{sources}", context.join("sources")),
        ("{initrd}", out.join("initrd.img")),
        ("{kernel}", out.join("kernel.elf")),
        ("{firmware}", out.join("firmware.bin")),
        ("{evidence}", out.to_path_buf()),
        (
            "{executable}",
            executable.unwrap_or(Path::new("")).to_path_buf(),
        ),
    ];
    let args: Vec<_> = args
        .iter()
        .map(|text| {
            let mut text = text.replace("{harts}", &harts.to_string());
            for (name, path) in &variables {
                text = text.replace(name, &path.to_string_lossy());
            }
            text
        })
        .collect();
    let program = args
        .first()
        .filter(|s| !s.is_empty())
        .ok_or("no declared runner; runtime acceptance unavailable")?;
    let mut command = Command::new(program);
    command.args(&args[1..]).current_dir(out);
    Ok(command)
}

fn run_deployment(config: &Config, context: &Path, out: &Path, name: &str) -> Result<(), String> {
    let deployment = config
        .deployments
        .get(name)
        .ok_or("undeclared deployment")?;
    if deployment.runner.is_empty() || deployment.harts.is_empty() {
        return Err("deployment has no declared runtime runner/Hart configuration".into());
    }
    build(config, context, out, name)?;
    for harts in &deployment.harts {
        let evidence = out.join(format!("run-{harts}"));
        std::fs::create_dir_all(&evidence).map_err(|e| e.to_string())?;
        let mut command = expand(&deployment.runner, context, out, *harts, None)?;
        command.current_dir(&evidence);
        run(&mut command, &evidence, deployment.timeout_seconds)?;
    }
    Ok(())
}

fn test_case(config: &Config, context: &Path, out: &Path, name: &str) -> Result<(), String> {
    match config
        .tests
        .get(name)
        .ok_or_else(|| format!("undeclared Unit/Cluster test {name}"))?
    {
        config::Test::Host {
            source,
            manifest,
            args,
            timeout_seconds,
        } => {
            let mut command = cargo(config);
            command
                .args([
                    "test",
                    "--locked",
                    "--target",
                    &config.host_target,
                    "--manifest-path",
                ])
                .arg(context.join("sources").join(source).join(manifest))
                .arg("--target-dir")
                .arg(context.join(format!("test-{name}")))
                .args(args)
                .current_dir(context);
            run(&mut command, out, *timeout_seconds)
        }
        config::Test::Target {
            source,
            manifest,
            package,
            bin,
            profile,
            features,
            runner,
            timeout_seconds,
        } => {
            let build = ImageBuild {
                image: "unit-test",
                manifest: literal(manifest.clone()),
                package: literal(package.clone()),
                bin: literal(bin.clone()),
                target: literal(config.target.clone()),
                profile: literal(profile.clone()),
                features: Box::leak(
                    features
                        .iter()
                        .cloned()
                        .map(literal)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
                default_features: true,
            };
            let executable = executables(
                &context.join("sources").join(source),
                &context.join(format!("test-{name}")),
                &[build],
            )?;
            let mut command = expand(runner, context, out, 0, Some(&executable[0].1))?;
            run(&mut command, out, *timeout_seconds)
        }
        config::Test::Scenario { deployment } => run_deployment(config, context, out, deployment),
    }
}

fn test(config: &Config, context: &Path, out: &Path, name: &str) -> Result<(), String> {
    if let Some(unit) = config.units.get(name) {
        if unit.tests.is_empty() {
            return Err(format!("Unit {name} declares no baseline tests"));
        }
        for selected in &unit.tests {
            let evidence = out.join(selected);
            std::fs::create_dir_all(&evidence).map_err(|e| e.to_string())?;
            test_case(config, context, &evidence, selected)?;
        }
        Ok(())
    } else {
        test_case(config, context, out, name)
    }
}

fn cluster_tests(
    config: &Config,
    name: &str,
    stack: &mut Vec<String>,
    picked: &mut Vec<String>,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("Cluster cycle at {name}"));
    }
    let cluster = config
        .clusters
        .get(name)
        .ok_or_else(|| format!("unknown Cluster {name}"))?;
    stack.push(name.to_string());
    for member in &cluster.members {
        if config.clusters.contains_key(member) {
            cluster_tests(config, member, stack, picked)?;
        } else if !config.units.contains_key(member) {
            return Err(format!("unknown Cluster member {member}"));
        }
    }
    for scenario in &cluster.scenarios {
        if !matches!(
            config.tests.get(scenario),
            Some(config::Test::Scenario { .. })
        ) {
            return Err(format!(
                "Cluster scenario {scenario} must name a declared test deployment"
            ));
        }
        if !picked.contains(scenario) {
            picked.push(scenario.clone());
        }
    }
    stack.pop();
    Ok(())
}

fn validate(config: &Config) -> Result<(), String> {
    if config.schema != 1 {
        return Err("unsupported configuration schema".into());
    }
    if config.host_target != "x86_64-unknown-linux-gnu"
        && config.host_target != "aarch64-unknown-linux-gnu"
    {
        return Err("host_target must explicitly select a supported host triple".into());
    }
    let mut names = Vec::new();
    for source in &config.sources {
        source::relative(Path::new(&source.name))?;
        if names.contains(&source.name) {
            return Err(format!("duplicate source {}", source.name));
        }
        if !source.git.starts_with("https://") || source.git.contains('@') {
            return Err("source URL must be credential-free HTTPS".into());
        }
        names.push(source.name.clone());
    }
    for workspace in &config.workspaces {
        if !names.contains(&workspace.source) {
            return Err("unknown workspace source".into());
        }
        source::relative(&workspace.manifest)?;
    }
    if !names.contains(&config.host.source) || !names.contains(&config.kernel.source) {
        return Err("unknown host/kernel source".into());
    }
    source::relative(&config.host.manifest)?;
    source::relative(&config.host.dependency_manifest)?;
    source::relative(Path::new(&config.kernel.manifest))?;
    for (name, deployment) in &config.deployments {
        source::relative(Path::new(name))?;
        if deployment.harts.contains(&0) {
            return Err("Hart count must be positive".into());
        }
    }
    for (name, test) in &config.tests {
        source::relative(Path::new(name))?;
        match test {
            config::Test::Host {
                source, manifest, ..
            } => {
                if !names.contains(source) {
                    return Err("unknown Unit test source".into());
                }
                source::relative(manifest)?;
            }
            config::Test::Target {
                source, manifest, ..
            } => {
                if !names.contains(source) {
                    return Err("unknown target test source".into());
                }
                source::relative(Path::new(manifest))?;
            }
            config::Test::Scenario { deployment } => {
                if !config.deployments.contains_key(deployment) {
                    return Err("unknown test deployment".into());
                }
            }
        }
    }
    for (name, unit) in &config.units {
        if config.tests.contains_key(name) || config.clusters.contains_key(name) {
            return Err(format!("Unit {name} conflicts with a test or Cluster name"));
        }
        source::relative(Path::new(name))?;
        source::relative(&unit.manifest)?;
        if !config
            .workspaces
            .iter()
            .any(|w| w.source == unit.source && w.manifest == unit.manifest)
        {
            return Err(format!(
                "Unit {name} workspace is not explicitly configured"
            ));
        }
        for test in &unit.tests {
            let source = match config.tests.get(test) {
                Some(config::Test::Host { source, .. } | config::Test::Target { source, .. }) => {
                    source
                }
                _ => {
                    return Err(format!(
                        "Unit {name} baseline {test} is missing or is a system scenario"
                    ));
                }
            };
            if source != &unit.source {
                return Err(format!("Unit {name} baseline source mismatch"));
            }
        }
    }
    for name in config.clusters.keys() {
        source::relative(Path::new(name))?;
        cluster_tests(config, name, &mut Vec::new(), &mut Vec::new())?;
        for package in config.clusters[name].shared_crates.values() {
            if !names.contains(&package.source) {
                return Err("unknown shared crate source".into());
            }
            source::relative(&package.path)?;
        }
    }
    Ok(())
}

/// Execute the same workflow used by the CLI. Locks, sources, substitutions,
/// build/run failures and evidence ownership are shared by all entrypoints.
pub fn execute(config_path: &Path, state: &Path, action: &[String]) -> Result<PathBuf, String> {
    let config_path = config_path.canonicalize().map_err(|e| e.to_string())?;
    let base = config_path.parent().unwrap();
    let config: Config =
        serde_json::from_slice(&std::fs::read(&config_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    validate(&config)?;
    let selected = std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default();
    if selected != config.toolchain
        && selected != format!("{}-{}", config.toolchain, config.host_target)
    {
        return Err(format!(
            "select the fixed toolchain first: RUSTUP_TOOLCHAIN={}",
            config.toolchain
        ));
    }
    let verb = action.first().map(String::as_str).unwrap_or("inspect");
    let state = source::resolved(state)?;
    if verb == "inspect" {
        // Inspect recorded contexts, including older tool snapshots, without
        // resolving sources or starting Cargo merely to view existing evidence.
        let contexts = state.join("contexts");
        if state.join("STOP_UNCONFIRMED").exists() {
            println!("reserved: {}", state.join("STOP_UNCONFIRMED").display());
        }
        if contexts.is_dir() {
            let mut paths = std::fs::read_dir(&contexts)
                .map_err(|e| e.to_string())?
                .map(|entry| entry.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            paths.sort();
            for context in paths {
                println!("evidence: {}", context.display());
                match std::fs::read_to_string(context.join("current.json")) {
                    Ok(record) => println!("published: {record}"),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
        }
        return Ok(state);
    }
    source::separate_state(&state, &base.join(&config.library))?;
    for input in load_overrides(&state)?.values() {
        source::separate_state(&state, input)?;
    }
    if verb == "override" && action.get(1).map(String::as_str) == Some("set") && action.len() == 4 {
        source::separate_state(&state, Path::new(&action[3]))?;
    }
    std::fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    let _lease = publication::leases(&[&state.join("operation")])?;
    if state.join("STOP_UNCONFIRMED").exists() {
        return Err(format!(
            "resource stop was not confirmed; context remains reserved: {}",
            state.join("STOP_UNCONFIRMED").display()
        ));
    }
    if verb == "override" {
        let mut overrides = load_overrides(&state)?;
        match action.get(1).map(String::as_str) {
            Some("set") if action.len() == 4 => {
                let name = &action[2];
                if !config.sources.iter().any(|s| &s.name == name) {
                    return Err("undeclared source override".into());
                }
                let path = Path::new(&action[3])
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                source::snapshot(&path, None)?;
                overrides.insert(name.clone(), path);
            }
            Some("clear") if action.len() == 3 => {
                overrides.remove(&action[2]);
            }
            Some("check") if action.len() == 2 => {
                return prepare(&config, base, &state);
            }
            _ => return Err("override set SOURCE CHECKOUT | clear SOURCE | check".into()),
        }
        json(&state.join("overrides.json"), &overrides)?;
        println!("{}", serde_json::to_string_pretty(&overrides).unwrap());
        return Ok(state.join("overrides.json"));
    }
    let context = prepare(&config, base, &state)?;
    if verb == "prepare" {
        return Ok(context);
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let out = context
        .join("attempts")
        .join(format!("{stamp}-{}", std::process::id()));
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let name = action.get(1).map(String::as_str).unwrap_or("");
    let result = match verb {
        "check" => host(&config, &context, &out, "check", &[]),
        "build" => build(&config, &context, &out, name),
        "run" => run_deployment(&config, &context, &out, name),
        "test" => test(&config, &context, &out, name),
        "cluster" => (|| {
            let mut tests = Vec::new();
            cluster_tests(&config, name, &mut Vec::new(), &mut tests)?;
            if tests.is_empty() {
                return Err("Cluster declares no collaboration scenarios".into());
            }
            for name in tests {
                let evidence = out.join(&name);
                std::fs::create_dir_all(&evidence).map_err(|e| e.to_string())?;
                test_case(&config, &context, &evidence, &name)?;
            }
            Ok(())
        })(),
        _ => Err(
            "expected prepare | check | build DEPLOYMENT | run DEPLOYMENT | test NAME | inspect"
                .into(),
        ),
    };
    complete_attempt(&state, &context, &out, action, result)
}

fn complete_attempt(
    state: &Path,
    context: &Path,
    out: &Path,
    action: &[String],
    result: Result<(), String>,
) -> Result<PathBuf, String> {
    let verb = action.first().map(String::as_str).unwrap_or("");
    let name = action.get(1).map(String::as_str).unwrap_or("");
    let result: Result<(), String> = (|| {
        result?;
        active()?;
        if verb == "build" || verb == "run" {
            // This file records preparation, not committed publication. Only
            // the single atomic pointer establishes deployment success.
            json(
                &out.join("outcome.json"),
                &serde_json::json!({
                    "action": action, "context": context, "phase": "prepared", "success": null,
                }),
            )?;
            active()?;
            json(
                &context.join("current.json"),
                &serde_json::json!({
                    "deployment": name, "artifacts": out, "action": verb,
                    "phase": "published", "success": true,
                }),
            )?;
        } else {
            json(
                &out.join("outcome.json"),
                &serde_json::json!({
                    "action": action, "context": context, "phase": "completed", "success": true,
                }),
            )?;
        }
        Ok(())
    })();
    if let Err(mut error) = result {
        // Reserve before any fallible diagnostic write; preserve the primary
        // execution/stop error even when evidence storage itself is failing.
        if error.contains("stop_confirmed=false")
            && let Err(reservation) =
                publication::write(&state.join("STOP_UNCONFIRMED"), error.as_bytes())
        {
            error = format!("{error}; reservation failed: {reservation}");
        }
        if let Err(record) = json(
            &out.join("outcome.json"),
            &serde_json::json!({
                "action": action, "context": context, "phase": "failed", "success": false, "error": error,
            }),
        ) {
            error = format!("{error}; outcome record failed: {record}");
        }
        return Err(format!("{error}; evidence: {}", out.display()));
    }
    Ok(out.to_path_buf())
}

#[cfg(test)]
mod tests;
