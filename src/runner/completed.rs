#[derive(Debug)]
pub struct Completed {
    pub directory: std::path::PathBuf,
    pub status: i32,
    pub serial: Vec<u8>,
}
