#!/usr/bin/env python3
"""Reject a mixed provider revision/ABI across the selected product components."""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
MANIFESTS = ["crates/build/Cargo.toml", "platform/riscv/Cargo.toml",
             "tests/platform/qemu/Cargo.toml", "tests/memory/Cargo.toml"]
LOCKS = ["Cargo.lock", "platform/riscv/Cargo.lock", "tests/platform/qemu/Cargo.lock",
         "tests/platform/hardware/Cargo.lock", "tests/memory/Cargo.lock"]

def providers(value):
    if isinstance(value, dict):
        for name, item in value.items():
            if name in ["kernel", "kernel-api", "kernel-client"] and isinstance(item, dict):
                yield item
            else:
                yield from providers(item)

def main():
    revisions = set()
    for manifest in MANIFESTS:
        dependencies = list(providers(tomllib.loads((ROOT / manifest).read_text())))
        assert dependencies, manifest
        for dependency in dependencies:
            assert dependency["git"] == "https://github.com/shuyra-arche/kernel.git", manifest
            assert "memory-v11" in dependency.get("features", []), manifest
            revisions.add(dependency["rev"])
    assert len(revisions) == 1, revisions
    revision, = revisions
    assert len(revision) == 40 and all(c in "0123456789abcdef" for c in revision)
    for lock in LOCKS:
        packages = tomllib.loads((ROOT / lock).read_text())["package"]
        sources = {p.get("source") for p in packages if p["name"] in ["kernel", "kernel-api", "kernel-client", "kernel-api-macros"]}
        assert sources == {f"git+https://github.com/shuyra-arche/kernel.git?rev={revision}#{revision}"}, (lock, sources)
    mode, commit, *_ = subprocess.check_output(
        ["git", "ls-files", "--stage", "components/kernel"], cwd=ROOT, text=True).split()
    assert mode == "160000" and commit == revision, commit
    print(f"PASS coordinated-version abi=11 provider={revision} manifests={len(MANIFESTS)} locks={len(LOCKS)} submodule=matched")

if __name__ == "__main__":
    main()
