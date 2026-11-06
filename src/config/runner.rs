#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runner {
    #[serde(default = "machine")]
    pub machine: String,
    #[serde(default = "memory")]
    pub memory: String,
    #[serde(default = "cpus")]
    pub cpus: u32,
    #[serde(default = "timeout")]
    pub timeout: u64,
}
fn machine() -> String {
    "virt".into()
}
fn memory() -> String {
    "256M".into()
}
fn cpus() -> u32 {
    1
}
fn timeout() -> u64 {
    60
}
