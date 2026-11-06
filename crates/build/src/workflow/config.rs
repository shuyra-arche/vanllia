use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub toolchain: String,
    pub host_target: String,
    pub target: String,
    pub sources: Vec<Source>,
    pub workspaces: Vec<Workspace>,
    pub host: Host,
    pub library: PathBuf,
    pub firmware: Firmware,
    pub kernel: Kernel,
    pub deployments: BTreeMap<String, Deployment>,
    #[serde(default)]
    pub tests: BTreeMap<String, Test>,
    #[serde(default)]
    pub clusters: BTreeMap<String, Cluster>,
    #[serde(default)]
    pub units: BTreeMap<String, Unit>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub name: String,
    pub git: String,
    pub rev: String,
    #[serde(default)]
    pub patches: Vec<PathBuf>,
    #[serde(default)]
    pub submodules: Vec<PathBuf>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub source: String,
    pub manifest: PathBuf,
    #[serde(default)]
    pub packages: BTreeMap<String, Package>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub source: String,
    pub path: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    pub source: String,
    pub manifest: PathBuf,
    pub package: String,
    pub bin: String,
    pub dependency_manifest: PathBuf,
    #[serde(default = "library_alias")]
    pub dependency_alias: String,
}

fn library_alias() -> String {
    "vanllia-build".to_string()
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Firmware {
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Kernel {
    pub source: String,
    pub manifest: String,
    pub package: String,
    pub bin: String,
    pub profile: String,
    pub features: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Deployment {
    pub host_args: Vec<String>,
    // A declared existing runner owns the framework, platform and semantic judge.
    // Placeholders: {sources}, {initrd}, {kernel}, {firmware}, {evidence}, {harts}.
    pub runner: Vec<String>,
    pub harts: Vec<u32>,
    pub timeout_seconds: u64,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Test {
    #[serde(rename = "host")]
    Host {
        source: String,
        manifest: PathBuf,
        args: Vec<String>,
        timeout_seconds: u64,
    },
    #[serde(rename = "target")]
    Target {
        source: String,
        manifest: String,
        package: String,
        bin: String,
        profile: String,
        features: Vec<String>,
        runner: Vec<String>,
        timeout_seconds: u64,
    },
    #[serde(rename = "scenario")]
    Scenario { deployment: String },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cluster {
    pub members: Vec<String>,
    pub scenarios: Vec<String>,
    #[serde(default)]
    pub shared_crates: BTreeMap<String, Package>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub source: String,
    pub manifest: PathBuf,
    pub tests: Vec<String>,
}
