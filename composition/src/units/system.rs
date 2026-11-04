//! Orchestration entry (prog-system) declaration. It is the scene entry, not a start item.

use unit::UnitFile;

pub static PROGRAM: UnitFile = UnitFile {
    name: "system",
    loader: unit::LoaderConfig::supervisor("system"),
    operator: unit::OperatorConfig { namespace: &[] },
    identity: unit::Identity {
        aliases: &[],
        grants: &[],
    },
    control: unit::ControlConfig {
        supervision: None,
        after: None,
        ready: unit::Ready::OnSpawn,
        ending: None,

        construction: &[],
        retirement: &[],
    },
    supply: unit::SupplyConfig { bindings: &[] },
};
