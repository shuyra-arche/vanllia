#[derive(Debug)]
pub enum Change {
    Use {
        name: String,
        path: std::path::PathBuf,
    },
    Reset(String),
}
