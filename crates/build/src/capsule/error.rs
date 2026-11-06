#[derive(Debug)]
pub(crate) enum Error {
    Format,
    Unsupported,
    Range,
    Permissions,
    Overlap,
    Entry,
    Memory,
}
