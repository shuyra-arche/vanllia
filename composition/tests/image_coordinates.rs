//! Frozen product Cargo coordinates and selected provider routes.
//!
//! Expectations come from frozen source plus the reviewed provider intakes,
//! not from the current coordinate producer.

use unit::ImageBuild;
use vanllia_composition::{IMAGE_BUILDS, image_builds, units};

const fn expected_image(
    image: &'static str,
    manifest: &'static str,
    package: &'static str,
    bin: &'static str,
) -> ImageBuild {
    ImageBuild {
        image,
        manifest,
        package,
        bin,
        features: &[],
        target: "riscv64gc-unknown-none-elf",
        profile: "release",
        default_features: true,
    }
}

const EXPECTED: &[ImageBuild] = &[
    expected_image(
        "hub",
        "components/driver/units/hub/server/Cargo.toml",
        "hub-server",
        "hub",
    ),
    expected_image(
        "router",
        "components/driver/units/router/server/Cargo.toml",
        "router-server",
        "router",
    ),
    expected_image(
        "pipe",
        "components/pipe/server/Cargo.toml",
        "pipe-server",
        "pipe",
    ),
    expected_image(
        "account",
        "components/session/units/account/server/Cargo.toml",
        "account-server",
        "account",
    ),
    expected_image(
        "terminal",
        "components/session/units/terminal/server/Cargo.toml",
        "terminal-server",
        "terminal",
    ),
    expected_image("system", "bootstrap/Cargo.toml", "vanllia-system", "system"),
    expected_image(
        "login",
        "components/session/units/login/server/Cargo.toml",
        "login-server",
        "login",
    ),
    expected_image(
        "uart",
        "components/driver/units/uart/server/Cargo.toml",
        "uart-server",
        "uart",
    ),
    expected_image(
        "rtc",
        "components/driver/units/rtc/server/Cargo.toml",
        "rtc-server",
        "rtc",
    ),
    expected_image(
        "daemon",
        "components/daemon/server/Cargo.toml",
        "daemon-server",
        "daemon",
    ),
    expected_image(
        "shell",
        "components/shell/server/entry/Cargo.toml",
        "shell-entry",
        "prog-shell",
    ),
    expected_image(
        "cat",
        "components/shell/cat/command/Cargo.toml",
        "cat",
        "prog-cat",
    ),
    expected_image(
        "emit",
        "components/shell/commands/Cargo.toml",
        "shell-command-images",
        "emit",
    ),
    expected_image(
        "upper",
        "components/shell/commands/Cargo.toml",
        "shell-command-images",
        "upper",
    ),
    expected_image(
        "fail",
        "components/shell/commands/Cargo.toml",
        "shell-command-images",
        "fail",
    ),
    expected_image(
        "spin",
        "components/shell/commands/Cargo.toml",
        "shell-command-images",
        "spin",
    ),
    expected_image(
        "workers",
        "components/shell/commands/Cargo.toml",
        "shell-command-images",
        "workers",
    ),
];

fn assert_non_manifest_fields(actual: &ImageBuild, expected: &ImageBuild) {
    assert_eq!(actual.image, expected.image);
    assert_eq!(actual.package, expected.package);
    assert_eq!(actual.bin, expected.bin);
    assert_eq!(actual.features, expected.features);
    assert_eq!(actual.target, expected.target);
    assert_eq!(actual.profile, expected.profile);
    assert_eq!(actual.default_features, expected.default_features);
}

#[test]
fn product_images_keep_complete_frozen_cargo_coordinates() {
    assert_eq!(IMAGE_BUILDS.len(), EXPECTED.len());
    for (actual, expected) in IMAGE_BUILDS.iter().zip(EXPECTED) {
        assert_non_manifest_fields(actual, expected);
        assert_eq!(actual.manifest, expected.manifest, "{}", expected.image);
    }
}

fn assert_owner_route(owner: &ImageBuild, prefix: &str) {
    let selected: Vec<_> = IMAGE_BUILDS
        .iter()
        .filter(|build| build.image == owner.image)
        .collect();
    assert_eq!(
        selected.len(),
        1,
        "{} must have one selected route",
        owner.image
    );
    assert_non_manifest_fields(selected[0], owner);
    assert_eq!(selected[0].manifest, format!("{prefix}{}", owner.manifest));
}

#[test]
fn selected_owner_builds_keep_their_fields_and_manifest_prefixes() {
    assert_owner_route(&units::daemon::BUILD, "components/daemon/");
    assert_owner_route(&units::login::BUILD, "components/session/units/login/");
    assert_owner_route(&units::uart::BUILD, "components/driver/units/uart/");
    assert_owner_route(&units::rtc::BUILD, "components/driver/units/rtc/");

    let builds = image_builds::shell_images_declaration::BUILDS;
    let names: Vec<_> = builds.iter().map(|build| build.image).collect();
    assert_eq!(
        names,
        ["shell", "cat", "emit", "upper", "fail", "spin", "workers"]
    );
    for build in builds {
        assert_owner_route(build, "components/shell/");
    }
    let aggregate = &image_builds::shell_images_declaration::BUILD;
    assert_non_manifest_fields(aggregate, &builds[0]);
    assert_eq!(aggregate.manifest, builds[0].manifest);
    assert_eq!(aggregate.image, units::shell::PROGRAM.loader.image);
}
