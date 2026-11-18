mod support;

use support::{Fixture, git};
use vanllia::{Error, testing};

#[test]
fn test_discovery_reads_provider_owned_cases_without_copying_them_into_vanllia() {
    let product = Fixture::new();
    product.project();
    let provider = Fixture::new();
    provider.write(
        "tests/roundtrip/test.toml",
        "schema = 1\nname = 'provider.roundtrip'\nunit = 'test.provider-roundtrip'\nexpected = 'clean'\ntimeout = 60\n",
    );
    provider.write(
        "tests/roundtrip/unit.toml",
        "schema = 1\nid = 'test.provider-roundtrip'\n[loader]\nimage = 'test-provider-roundtrip'\n",
    );
    git(&provider.root, &["init", "-q"]);
    product.write(
        "vanllia.toml",
        &format!(
            "schema = 1\nunits = ['probe']\n[boot]\nsystem = 'system'\n[sources.provider]\npath = {:?}\n",
            provider.root.to_str().unwrap()
        ),
    );

    let cases = testing::cases(&product.root).unwrap();
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].name, "provider.roundtrip");
    assert_eq!(cases[0].unit, "test.provider-roundtrip");
    assert!(!product.root.join("tests/roundtrip").exists());

    // Listing a provider's test must not build a product or require firmware.
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_vanllia"))
        .current_dir(&product.root)
        .args(["test", "--", "--list"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("provider.roundtrip"));
    assert!(!product.root.join(".vanllia/artifacts").exists());
}

#[test]
fn test_discovery_rejects_duplicate_names_across_provider_sources() {
    let product = Fixture::new();
    product.project();
    let first = Fixture::new();
    let second = Fixture::new();
    for source in [&first, &second] {
        source.write(
            "tests/probe/test.toml",
            "schema = 1\nname = 'duplicate.probe'\nunit = 'test.probe'\n",
        );
        git(&source.root, &["init", "-q"]);
    }
    product.write(
        "vanllia.toml",
        &format!(
            "schema = 1\n[boot]\nsystem = 'system'\n[sources.first]\npath = {:?}\n[sources.second]\npath = {:?}\n",
            first.root.to_str().unwrap(),
            second.root.to_str().unwrap()
        ),
    );
    assert!(matches!(
        testing::cases(&product.root),
        Err(Error::Declaration { message, .. }) if message.contains("duplicate test name")
    ));
}

#[test]
fn test_discovery_rejects_invalid_provider_declarations() {
    let product = Fixture::new();
    product.project();
    product.write(
        "tests/invalid/test.toml",
        "schema = 1\nname = 'invalid.case'\nunit = ''\ntimeout = 0\n",
    );
    assert!(matches!(
        testing::cases(&product.root),
        Err(Error::Declaration { path, .. }) if path.ends_with("tests/invalid/test.toml")
    ));
}
