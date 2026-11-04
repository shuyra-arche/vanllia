//! Daemon UnitFile.

use unit::{Ending, UnitFile};

pub static PROGRAM: UnitFile = UnitFile {
    name: "daemon",
    loader: unit::LoaderConfig::user("daemon"),
    operator: unit::OperatorConfig::DEFAULT,
    identity: unit::Identity::DEFAULT,
    control: unit::ControlConfig {
        supervision: Some("supervised"),
        after: Some(&[
            unit::Dependency::Bootstrap(unit::Bootstrap::Operator),
            unit::Dependency::Target("supervised"),
        ]),
        ending: Some(Ending::Resident),
        ready: unit::Ready::Signal(unit::READY_MARK),
        construction: &[],
        retirement: &[],
    },
    supply: unit::SupplyConfig::DEFAULT,
};
