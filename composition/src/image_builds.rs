//! Explicit Cargo coordinates for the 17 product images.

use unit::ImageBuild;

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

macro_rules! legacy_image {
    ($image:literal) => {
        ImageBuild {
            image: $image,
            manifest: "legacy/sqware/programs/Cargo.toml",
            package: "programs",
            bin: concat!("prog-", $image),
            features: &[],
            target: TARGET,
            profile: PROFILE,
            default_features: true,
        }
    };
}

pub const IMAGE_BUILDS: &[ImageBuild] = &[
    HUB,
    ROUTER,
    PIPE,
    ACCOUNT,
    TERMINAL,
    SYSTEM,
    legacy_image!("login"),
    legacy_image!("uart"),
    legacy_image!("rtc"),
    DAEMON,
    legacy_image!("shell"),
    legacy_image!("cat"),
    legacy_image!("emit"),
    legacy_image!("upper"),
    legacy_image!("fail"),
    legacy_image!("spin"),
    legacy_image!("workers"),
];
