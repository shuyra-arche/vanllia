#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Created,
    Ready,
    Exited(unit_contract::event::Exit),
    Retired(unit_contract::event::Exit),
}
