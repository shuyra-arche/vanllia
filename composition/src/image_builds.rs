//! Explicit Cargo coordinates for the 17 product images.

use unit::ImageBuild;

// Host equivalence reads the provider's ordered aggregate through composition.
pub use shell_images_declaration;

const TARGET: &str = "riscv64gc-unknown-none-elf";
const PROFILE: &str = "release";

const HUB: ImageBuild = ImageBuild {
    image: driver_cluster::hub::BUILD.image,
    manifest: "components/driver/units/hub/server/Cargo.toml",
    package: driver_cluster::hub::BUILD.package,
    bin: driver_cluster::hub::BUILD.bin,
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const ROUTER: ImageBuild = ImageBuild {
    image: driver_cluster::router::BUILD.image,
    manifest: "components/driver/units/router/server/Cargo.toml",
    package: driver_cluster::router::BUILD.package,
    bin: driver_cluster::router::BUILD.bin,
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const PIPE: ImageBuild = ImageBuild {
    image: pipe_declaration::BUILD.image,
    manifest: "components/pipe/server/Cargo.toml",
    package: pipe_declaration::BUILD.package,
    bin: pipe_declaration::BUILD.bin,
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const ACCOUNT: ImageBuild = ImageBuild {
    image: session_cluster::account::BUILD.image,
    manifest: "components/session/units/account/server/Cargo.toml",
    package: session_cluster::account::BUILD.package,
    bin: session_cluster::account::BUILD.bin,
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const TERMINAL: ImageBuild = ImageBuild {
    image: session_cluster::terminal::BUILD.image,
    manifest: "components/session/units/terminal/server/Cargo.toml",
    package: session_cluster::terminal::BUILD.package,
    bin: session_cluster::terminal::BUILD.bin,
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const LOGIN: ImageBuild = ImageBuild {
    manifest: "components/session/units/login/server/Cargo.toml",
    ..session_cluster::login::BUILD
};

const UART: ImageBuild = ImageBuild {
    manifest: "components/driver/units/uart/server/Cargo.toml",
    ..driver_cluster::uart::BUILD
};

const RTC: ImageBuild = ImageBuild {
    manifest: "components/driver/units/rtc/server/Cargo.toml",
    ..driver_cluster::rtc::BUILD
};

const DAEMON: ImageBuild = ImageBuild {
    manifest: "components/daemon/server/Cargo.toml",
    ..daemon_declaration::BUILD
};

const SYSTEM: ImageBuild = ImageBuild {
    image: "system",
    manifest: "bootstrap/Cargo.toml",
    package: "vanllia-system",
    bin: "system",
    features: &[],
    target: TARGET,
    profile: PROFILE,
    default_features: true,
};

const SHELL_MANIFEST: &str = "components/shell/server/entry/Cargo.toml";
const CAT_MANIFEST: &str = "components/shell/cat/command/Cargo.toml";
const SHELL_COMMANDS_MANIFEST: &str = "components/shell/commands/Cargo.toml";

const SHELL: ImageBuild = ImageBuild {
    manifest: SHELL_MANIFEST,
    ..shell_images_declaration::BUILDS[0]
};

const CAT: ImageBuild = ImageBuild {
    manifest: CAT_MANIFEST,
    ..shell_images_declaration::BUILDS[1]
};

const EMIT: ImageBuild = ImageBuild {
    manifest: SHELL_COMMANDS_MANIFEST,
    ..shell_images_declaration::BUILDS[2]
};

const UPPER: ImageBuild = ImageBuild {
    manifest: SHELL_COMMANDS_MANIFEST,
    ..shell_images_declaration::BUILDS[3]
};

const FAIL: ImageBuild = ImageBuild {
    manifest: SHELL_COMMANDS_MANIFEST,
    ..shell_images_declaration::BUILDS[4]
};

const SPIN: ImageBuild = ImageBuild {
    manifest: SHELL_COMMANDS_MANIFEST,
    ..shell_images_declaration::BUILDS[5]
};

const WORKERS: ImageBuild = ImageBuild {
    manifest: SHELL_COMMANDS_MANIFEST,
    ..shell_images_declaration::BUILDS[6]
};

pub const IMAGE_BUILDS: &[ImageBuild] = &[
    HUB,
    ROUTER,
    PIPE,
    ACCOUNT,
    TERMINAL,
    SYSTEM,
    LOGIN,
    UART,
    RTC,
    DAEMON,
    SHELL,
    CAT,
    EMIT,
    UPPER,
    FAIL,
    SPIN,
    WORKERS,
];
