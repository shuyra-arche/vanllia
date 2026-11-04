//! Host-only equivalence against the PR55 legacy product composition.
//!
//! These checks use declarations only; they do not inspect legacy build paths.

use vanllia_build::{image_inventory, startup_order};

const SHELL_COMMAND_IMAGES: [&str; 5] = ["emit", "upper", "fail", "spin", "workers"];
const REMAINING_LEGACY_IMAGES: [&str; 5] = ["login", "uart", "rtc", "shell", "cat"];
const LEGACY_MANIFEST: &str = "legacy/sqware/programs/Cargo.toml";
const SHELL_COMMANDS_MANIFEST: &str = "components/shell/commands/Cargo.toml";
const NO_PORTS: &[&str] = &[];

fn product(deployments: &'static [unit::Deployment]) -> &'static unit::Deployment {
    deployments
        .iter()
        .find(|deployment| deployment.name == "product")
        .expect("product deployment")
}

#[test]
fn product_deployment_fields_match_pr55() {
    let ours = product(vanllia_composition::DEPLOYMENTS);
    let legacy = product(legacy_composition::DEPLOYMENTS);

    assert_eq!(ours.name, legacy.name);
    assert_eq!(ours.entry, legacy.entry);
    assert_eq!(ours.start, legacy.start);
    assert_eq!(ours.guarded, legacy.guarded);
    assert_eq!(ours.resources, legacy.resources);
    assert_eq!(ours.sources, legacy.sources);
    assert_eq!(ours.targets.len(), legacy.targets.len());
    for (ours, legacy) in ours.targets.iter().zip(legacy.targets) {
        assert_eq!(ours.name, legacy.name);
        assert_eq!(ours.members, legacy.members);
    }
    assert_eq!(
        format!("{:?}", ours.bootstrap),
        format!("{:?}", legacy.bootstrap)
    );
}

#[test]
fn product_inventory_and_startup_match_pr55() {
    let ours = product(vanllia_composition::DEPLOYMENTS);
    let legacy = product(legacy_composition::DEPLOYMENTS);

    let ours_inventory =
        image_inventory(vanllia_composition::PROGRAMS, ours, vanllia_composition::IMAGE_BUILDS)
            .expect("vanllia product inventory");
    let legacy_inventory =
        image_inventory(legacy_composition::PROGRAMS, legacy, legacy_composition::IMAGE_BUILDS)
            .expect("PR55 product inventory");

    assert_eq!(vanllia_composition::IMAGE_BUILDS.len(), 17);
    assert_eq!(ours_inventory.len(), 17);
    assert_eq!(legacy_inventory.len(), 17);

    let ours_names: Vec<&str> = ours_inventory.iter().map(|(name, _)| *name).collect();
    let legacy_names: Vec<&str> = legacy_inventory.iter().map(|(name, _)| *name).collect();
    let coordinate_names: Vec<&str> = vanllia_composition::IMAGE_BUILDS
        .iter()
        .map(|build| build.image)
        .collect();
    assert_eq!(
        coordinate_names,
        [
            "hub", "router", "pipe", "account", "terminal", "system", "login", "uart", "rtc",
            "daemon", "shell", "cat", "emit", "upper", "fail", "spin", "workers",
        ]
        .to_vec()
    );
    assert_eq!(ours_names, legacy_names);
    for ((_, ours_kind), (_, legacy_kind)) in ours_inventory.iter().zip(&legacy_inventory) {
        assert_eq!(
            format!("{ours_kind:?}"),
            format!("{legacy_kind:?}"),
            "program kind differs"
        );
    }

    assert_eq!(
        startup_order(vanllia_composition::PROGRAMS, ours).expect("vanllia startup order"),
        startup_order(legacy_composition::PROGRAMS, legacy).expect("PR55 startup order")
    );
}

fn image_build(image: &str) -> &'static unit::ImageBuild {
    vanllia_composition::IMAGE_BUILDS
        .iter()
        .find(|build| build.image == image)
        .expect("product image build")
}

fn assert_same_non_manifest_coordinates(build: &unit::ImageBuild, expected: &unit::ImageBuild) {
    assert_eq!(build.image, expected.image);
    assert_eq!(build.package, expected.package);
    assert_eq!(build.bin, expected.bin);
    assert_eq!(build.features, expected.features);
    assert_eq!(build.target, expected.target);
    assert_eq!(build.profile, expected.profile);
    assert_eq!(build.default_features, expected.default_features);
}

fn provider_builds() -> &'static [unit::ImageBuild] {
    vanllia_composition::image_builds::shell_command_images_declaration::BUILDS
}

fn provider_command_build(image: &str) -> &'static unit::ImageBuild {
    provider_builds()
        .iter()
        .find(|build| build.image == image)
        .expect("provider command image build")
}

#[test]
fn daemon_is_provider_unit_and_build() {
    let provider = &vanllia_composition::units::daemon::UNIT;
    let legacy = &legacy_composition::units::daemon::PROGRAM;

    let slot = vanllia_composition::PROGRAMS
        .iter()
        .position(|program| program.name == "daemon")
        .expect("daemon registry slot");
    assert!(core::ptr::eq(vanllia_composition::PROGRAMS[slot], provider));

    assert_eq!(provider.name, legacy.name);
    assert_eq!(provider.loader.image, legacy.loader.image);
    assert_eq!(provider.loader.space, legacy.loader.space);
    assert_eq!(provider.loader.args, legacy.loader.args);
    assert_eq!(provider.loader.stack, legacy.loader.stack);
    assert_eq!(
        provider.operator.namespace.len(),
        legacy.operator.namespace.len()
    );
    assert_eq!(provider.identity.aliases, legacy.identity.aliases);
    assert_eq!(provider.identity.grants.len(), legacy.identity.grants.len());
    assert_eq!(provider.control.supervision, legacy.control.supervision);
    assert_eq!(provider.control.after, legacy.control.after);
    assert_eq!(provider.control.ending, legacy.control.ending);
    assert_eq!(provider.control.ready, legacy.control.ready);
    assert!(provider.control.construction == legacy.control.construction);
    assert_eq!(provider.control.retirement, legacy.control.retirement);
    assert_eq!(provider.supply.bindings, legacy.supply.bindings);

    let daemon = image_build("daemon");
    let provider_build = &vanllia_composition::units::daemon::BUILD;
    assert_same_non_manifest_coordinates(daemon, provider_build);
    assert_eq!(daemon.manifest, "components/daemon/server/Cargo.toml");
    assert_ne!(daemon.manifest, provider_build.manifest);

    assert_eq!(vanllia_composition::IMAGE_BUILDS.len(), 17);
    assert_eq!(
        vanllia_composition::IMAGE_BUILDS
            .iter()
            .filter(|build| build.image == "daemon")
            .count(),
        1
    );
    let legacy_images: Vec<&str> = vanllia_composition::IMAGE_BUILDS
        .iter()
        .filter(|build| build.manifest == LEGACY_MANIFEST)
        .map(|build| build.image)
        .collect();
    assert_eq!(legacy_images, REMAINING_LEGACY_IMAGES.to_vec());
    assert_eq!(provider_builds().len(), SHELL_COMMAND_IMAGES.len());

    for build in vanllia_composition::IMAGE_BUILDS
        .iter()
        .filter(|build| build.image != "daemon")
    {
        if build.image == "system" {
            assert_eq!(build.manifest, "bootstrap/Cargo.toml");
            continue;
        }
        if SHELL_COMMAND_IMAGES.contains(&build.image) {
            let provider_build = provider_command_build(build.image);
            assert_same_non_manifest_coordinates(build, provider_build);
            assert_eq!(build.manifest, SHELL_COMMANDS_MANIFEST);
            assert_eq!(provider_build.manifest, "commands/Cargo.toml");
            assert_ne!(build.manifest, provider_build.manifest);

            let pr55 = legacy_composition::IMAGE_BUILDS
                .iter()
                .find(|pr55| pr55.image == build.image)
                .expect("PR55 image build");
            assert_ne!(build.package, pr55.package);
            assert_ne!(build.bin, pr55.bin);
            continue;
        }
        let pr55 = legacy_composition::IMAGE_BUILDS
            .iter()
            .find(|pr55| pr55.image == build.image)
            .expect("PR55 image build");
        assert_eq!(build.package, pr55.package);
        assert_eq!(build.bin, pr55.bin);
        assert_eq!(build.features, pr55.features);
        assert_eq!(build.target, pr55.target);
        assert_eq!(build.profile, pr55.profile);
        assert_eq!(build.default_features, pr55.default_features);
    }
}

#[test]
fn shell_command_catalogue_keeps_frozen_ports_and_bindings() {
    let commands = vanllia_composition::units::shell::COMMANDS;
    let expected: [(&str, &[&str]); 6] = [
        ("cat", &["source", "copy"]),
        ("emit", &["records"]),
        ("upper", &["source", "result"]),
        ("fail", NO_PORTS),
        ("spin", NO_PORTS),
        ("workers", &["ticks"]),
    ];

    assert_eq!(commands.len(), expected.len());
    for (command, (name, ports)) in commands.iter().zip(expected) {
        assert_eq!(command.name, name);
        let actual: Vec<&str> = command.ports.iter().map(|(port, _)| *port).collect();
        assert_eq!(actual, ports.to_vec());

        let legacy = legacy_composition::units::shell::COMMANDS
            .iter()
            .find(|legacy| legacy.name == name)
            .expect("frozen command declaration");
        assert_eq!(command.ports, legacy.ports);
        assert_eq!(command.binding, legacy.binding);
    }
    assert_eq!(
        vanllia_composition::units::shell::BINDINGS,
        legacy_composition::units::shell::BINDINGS
    );

    let provider_images: Vec<&str> = provider_builds()
        .iter()
        .map(|build| build.image)
        .collect();
    assert_eq!(provider_images, SHELL_COMMAND_IMAGES.to_vec());
    for build in provider_builds() {
        assert!(commands.iter().any(|command| command.name == build.image));
        assert_eq!(build.bin, build.image);
    }
}
