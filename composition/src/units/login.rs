use unit::{Ending, UnitFile};

pub static PROGRAM: UnitFile = UnitFile {
    name: "login",
    loader: unit::LoaderConfig::user("login"),
    operator: unit::OperatorConfig { namespace: &[] },
    identity: unit::Identity {
        aliases: &[],
        grants: &[],
    },
    control: unit::ControlConfig {
        supervision: None,
        after: Some(&[
            unit::Dependency::Bootstrap(unit::Bootstrap::Operator),
            unit::Dependency::Unit("terminal"),
            unit::Dependency::Unit("account"),
            unit::Dependency::Target(unit::SCENE),
        ]),
        ready: unit::Ready::OnSpawn,
        ending: Some(Ending::Told),

        construction: &[],
        retirement: &[],
    },
    supply: unit::SupplyConfig { bindings: &[] },
};
