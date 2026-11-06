#[derive(Clone, Debug)]
pub enum Selection {
    Default,
    Explicit(Vec<String>),
}
