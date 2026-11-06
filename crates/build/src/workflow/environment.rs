use std::collections::BTreeMap;
use std::ffi::OsString;

// Capture compilation controls, never whole environments or credential prefixes.
pub(super) fn selected(
    variables: impl Iterator<Item = (OsString, OsString)>,
) -> Result<BTreeMap<String, String>, String> {
    let mut selected = BTreeMap::new();
    for (name, value) in variables {
        let Some(name) = name.to_str() else {
            continue;
        };
        let compiler = [
            "CARGO",
            "RUSTC",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_RUSTFLAGS",
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
            "CARGO_BUILD_TARGET",
            "CARGO_TARGET_DIR",
        ]
        .contains(&name);
        let target = name.starts_with("CARGO_TARGET_")
            && ["_RUSTFLAGS", "_LINKER", "_RUNNER"]
                .iter()
                .any(|suffix| name.ends_with(suffix));
        let profile = name.starts_with("CARGO_PROFILE_")
            && [
                "_OPT_LEVEL",
                "_DEBUG",
                "_DEBUG_ASSERTIONS",
                "_OVERFLOW_CHECKS",
                "_LTO",
                "_PANIC",
                "_CODEGEN_UNITS",
                "_RPATH",
                "_STRIP",
                "_SPLIT_DEBUGINFO",
                "_INCREMENTAL",
            ]
            .iter()
            .any(|suffix| name.ends_with(suffix));
        if compiler || target || profile {
            let value = value
                .into_string()
                .map_err(|_| format!("non-UTF8 build variable {name}"))?;
            selected.insert(name.to_string(), value);
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compilation_controls_are_recorded_without_credentials() {
        let vars = [
            ("RUSTFLAGS", "-Cdebuginfo=1"),
            ("CARGO_TARGET_RISCV64GC_UNKNOWN_NONE_ELF_LINKER", "rust-lld"),
            ("CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS", "true"),
            ("GH_TOKEN", "secret"),
            ("CARGO_REGISTRIES_PRIVATE_TOKEN", "secret"),
            ("CARGO_PROFILE_RELEASE_TOKEN", "secret"),
        ]
        .into_iter()
        .map(|(name, value)| (name.into(), value.into()));
        let recorded = selected(vars).unwrap();
        assert_eq!(recorded.len(), 3);
        assert_eq!(recorded["RUSTFLAGS"], "-Cdebuginfo=1");
        assert!(!recorded.values().any(|value| value == "secret"));
        let changed = selected([("RUSTFLAGS".into(), "-Cdebuginfo=2".into())].into_iter()).unwrap();
        assert_ne!(
            serde_json::to_vec(&recorded).unwrap(),
            serde_json::to_vec(&changed).unwrap()
        );
    }
}
