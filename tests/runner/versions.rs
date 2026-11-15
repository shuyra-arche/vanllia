use kernel_verification::*;
use std::{collections::BTreeSet, fs, path::Path};
const PROVIDERS: [&str; 3] = ["kernel", "kernel-api", "kernel-client"];
fn providers<'a>(value: &'a toml::Value, found: &mut Vec<&'a toml::Value>) {
    if let Some(table) = value.as_table() {
        for (name, item) in table {
            if PROVIDERS.contains(&name.as_str()) && item.is_table() {
                found.push(item);
            } else {
                providers(item, found);
            }
        }
    }
}
pub fn run(root: &Path) -> Result<()> {
    let mut revisions = BTreeSet::new();
    for path in [
        "crates/build/Cargo.toml",
        "platform/riscv/Cargo.toml",
        "tests/platform/qemu/Cargo.toml",
        "tests/memory/Cargo.toml",
    ] {
        let value: toml::Value = toml::from_str(&fs::read_to_string(root.join(path))?)?;
        let mut deps = vec![];
        providers(&value, &mut deps);
        require(!deps.is_empty(), format!("no providers in {path}"))?;
        for dep in deps {
            require(
                dep["git"].as_str() == Some("https://github.com/shuyra-arche/kernel.git"),
                format!("unexpected provider URL in {path}"),
            )?;
            require(
                !dep.get("features")
                    .and_then(|v| v.as_array())
                    .is_some_and(|v| v.iter().any(|x| x.as_str() == Some("memory-v11"))),
                "retired ABI feature",
            )?;
            let pin = dep["rev"].as_str().ok_or("provider requires rev")?;
            require(
                pin.len() == 40
                    && pin
                        .bytes()
                        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
                "provider requires full SHA",
            )?;
            revisions.insert(pin.to_owned());
        }
    }
    require(revisions.len() == 1, "mixed provider revisions")?;
    let pin = revisions.first().unwrap();
    let expected = format!("git+https://github.com/shuyra-arche/kernel.git?rev={pin}#{pin}");
    for path in [
        "Cargo.lock",
        "platform/riscv/Cargo.lock",
        "tests/platform/qemu/Cargo.lock",
        "tests/platform/hardware/Cargo.lock",
        "tests/memory/Cargo.lock",
    ] {
        let value: toml::Value = toml::from_str(&fs::read_to_string(root.join(path))?)?;
        let packages = value["package"].as_array().ok_or("invalid Cargo lock")?;
        let mut found = false;
        for package in packages {
            let name = package["name"].as_str().unwrap_or("");
            if PROVIDERS.contains(&name) || name == "kernel-api-macros" {
                found = true;
                require(
                    package.get("source").and_then(|v| v.as_str()) == Some(&expected),
                    format!("mixed provider in {path}"),
                )?;
            }
        }
        require(found, format!("missing provider in {path}"))?;
    }
    let entry = output(git(root).args(["ls-files", "--stage", "components/kernel"]))?;
    let mut fields = entry.split_whitespace();
    require(
        fields.next() == Some("160000") && fields.next() == Some(pin),
        "Kernel gitlink does not match providers",
    )?;
    println!("PASS coordinated-version abi=11 provider={pin}");
    Ok(())
}
