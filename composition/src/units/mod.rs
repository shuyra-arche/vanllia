//! Product UnitFiles: published declarations plus the four still-local ones.

pub use daemon_declaration as daemon;
pub use driver_cluster::{hub, router};
pub use pipe_declaration as pipe;
pub use session_cluster::{account, terminal};
pub use shell_declaration as shell;
use unit::UnitFile;

pub mod login;
pub mod rtc;
pub mod system;
pub mod uart;

/// Product declarations in Source registration order.
#[rustfmt::skip]
pub const PROGRAMS: &[&UnitFile] = &[
    &hub::UNIT,
    &daemon::UNIT,
    &pipe::UNIT,
    &account::UNIT,
    &terminal::UNIT,
    &login::PROGRAM,
    &shell::PROGRAM,
    &router::UNIT,
    &uart::PROGRAM,
    &rtc::PROGRAM,
    &system::PROGRAM,
];
