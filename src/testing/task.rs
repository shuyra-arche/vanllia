mod phase;
pub use phase::Phase;

#[derive(Clone, Debug)]
pub struct Task {
    pub unit: String,
    pub team: u64,
    pub phase: Phase,
}
