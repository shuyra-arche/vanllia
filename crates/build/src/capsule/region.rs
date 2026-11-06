#[derive(Debug)]
pub(crate) struct Region {
    pub(crate) va: usize,
    pub(crate) size: usize,
    pub(crate) data_size: usize,
    pub(crate) flags: u64,
    pub(crate) file_offset: usize,
    pub(crate) file_size: usize,
    pub(crate) prefix: usize,
}
