//! Product UnitFiles: published declarations plus the still-local bootstrap.

pub use daemon_declaration as daemon;
pub use driver_cluster::{hub, router, rtc, uart};
pub use pipe_declaration as pipe;
pub use session_cluster::{account, login, terminal};
pub use shell_declaration as shell;
use unit::UnitFile;

pub mod system;

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
    &uart::UNIT,
    &rtc::UNIT,
    &system::PROGRAM,
];
