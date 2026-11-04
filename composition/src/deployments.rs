//! The exact M1 product scene. Runtime and host consume this one declaration.

use unit::{Bootstrap, Deployment, Target, scene_target};

pub const PRODUCT: Deployment = Deployment {
    name: "product",
    entry: "system",
    start: &[
        "hub",
        "pipe",
        "account",
        "terminal",
        "login",
        "router",
        "uart",
        "rtc",
        "daemon",
    ],
    guarded: &[],
    resources: &["shell", "cat", "emit", "upper", "fail", "spin", "workers"],
    targets: &[
        scene_target(&[
            "hub", "pipe", "account", "terminal", "router", "uart", "rtc", "daemon",
        ]),
        Target {
            name: "supervised",
            members: &["rtc"],
        },
    ],
    bootstrap: &[Bootstrap::Operator, Bootstrap::Identity],
    sources: &[],
};

pub const DEPLOYMENTS: &[Deployment] = &[PRODUCT];
