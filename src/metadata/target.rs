use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Target {
    pub name: String,
    pub kind: Vec<String>,
}
