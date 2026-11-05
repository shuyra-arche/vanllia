#!/usr/bin/env python3
"""Record committed inputs; do not fetch, build, assign versions or certify a release."""

import argparse
import configparser
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tomllib


def git(repo, *args):
    return subprocess.check_output([
        "git", "--no-lazy-fetch", "--no-replace-objects", "--no-optional-locks",
        "-C", str(repo), *args,
    ])


def tree(repo, commit):
    entries = {}
    for record in git(repo, "ls-tree", "-rz", commit).split(b"\0"):
        if record:
            header, path = record.split(b"\t", 1)
            mode, kind, sha = header.decode().split()
            entries[path.decode()] = (mode, kind, sha)
    return entries


def blob(repo, commit, path):
    return git(repo, "show", f"{commit}:{path}")


def fingerprint(data, sha):
    return {"blob": sha, "sha256": hashlib.sha256(data).hexdigest()}


def inventory(root, commit):
    records = []

    def visit(relative, sha, url, historical=False):
        repo = root / relative
        # An uninitialized submodule must not silently resolve to its parent's Git repo.
        top = Path(git(repo, "rev-parse", "--show-toplevel").decode().strip()).resolve()
        if top != repo.resolve():
            raise ValueError(f"initialize the exact submodule first: {relative}")
        if git(repo, "cat-file", "-t", sha).strip() != b"commit":
            raise ValueError(f"expected a commit object at {relative}: {sha}")
        entries = tree(repo, sha)
        item = {
            "path": relative.as_posix(),
            "repository": url,
            "commit": sha,
            "tree": git(repo, "rev-parse", f"{sha}^{{tree}}").decode().strip(),
            "role": "historical-dev-fixture" if historical else "source-inventory",
            "release_version": None,
            "manifests": [],
            "locks": [],
            "toolchains": [],
            "build_config_files": [],
            "gitlinks": [],
        }
        records.append(item)
        module_urls = {}
        if ".gitmodules" in entries:
            modules = configparser.ConfigParser(interpolation=None)
            modules.read_string(blob(repo, sha, ".gitmodules").decode())
            for section in modules.sections():
                path = modules[section]["path"]
                if path in module_urls:
                    raise ValueError(f"duplicate submodule path at {relative}: {path}")
                module_urls[path] = modules[section]["url"]
        for path, (mode, kind, object_sha) in sorted(entries.items()):
            if mode == "160000":
                child = PurePosixPath(path)
                if child.is_absolute() or ".." in child.parts:
                    raise ValueError(f"unsafe gitlink path: {path}")
                child_url = module_urls[path]
                item["gitlinks"].append({
                    "path": path, "commit": object_sha, "repository": child_url,
                    "expanded": not historical,
                })
                if not historical:
                    child_path = relative / path
                    visit(child_path, object_sha, child_url, child_path == Path("legacy/sqware"))
                continue
            if kind != "blob":
                continue
            if historical and path not in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}:
                continue
            name = PurePosixPath(path).name
            if path == "scripts/build.nu" or path.endswith(".cargo/config.toml"):
                data = blob(repo, sha, path)
                item["build_config_files"].append({"path": path, **fingerprint(data, object_sha)})
                continue
            if name not in {"Cargo.toml", "Cargo.lock", "rust-toolchain.toml"}:
                continue
            data = blob(repo, sha, path)
            parsed = tomllib.loads(data.decode())
            record = {"path": path, **fingerprint(data, object_sha)}
            if name == "Cargo.lock":
                record["package_count"] = len(parsed.get("package", []))
                git_packages = [
                    {"name": p["name"], "version": p["version"], "source": p["source"]}
                    for p in parsed.get("package", [])
                    if p.get("source", "").startswith("git+")
                ]
                record["git_package_count"] = len(git_packages)
                if relative == Path("."):
                    record["git_packages"] = git_packages
                item["locks"].append(record)
            elif name == "Cargo.toml":
                package = parsed.get("package", {})
                record.update({
                    "package": package.get("name"),
                    "declared_version": package.get("version"),
                    "workspace_version": parsed.get("workspace", {}).get("package", {}).get("version"),
                })
                item["manifests"].append(record)
            else:
                record["toolchain"] = parsed.get("toolchain")
                item["toolchains"].append(record)

    visit(Path("."), commit, "https://github.com/shuyra-arche/vanllia.git")
    return {
        "schema_version": 1,
        "kind": "committed-input-inventory",
        "is_release_manifest": False,
        "is_dependency_graph": False,
        "vanllia_commit": commit,
        "vanllia_version": None,
        "release_approved": False,
        "scope": {
            "source": "Git objects, not worktree files; no network or Cargo resolution",
            "locks": "all tracked locks in expanded sources, not a selected production closure",
            "lock_content": "full Git package identities for the root lock; other locks use content hashes/counts",
            "historical": "legacy/sqware recorded at top level; its child gitlinks are not expanded",
            "package_versions": "declared metadata only; workspace references stay unresolved",
        },
        "repositories": records,
        "build_records": [],
        "firmware_selection": None,
        "runtime_evidence": [],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--commit", required=True, help="full 40-hex commit, never a branch or tag")
    parser.add_argument("--output", type=Path, help="new file only; default is stdout")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.commit):
        parser.error("--commit must be a full lowercase 40-hex commit")
    try:
        data = json.dumps(inventory(args.root.resolve(), args.commit), indent=2) + "\n"
        if args.output:
            with args.output.open("x") as stream:
                stream.write(data)
        else:
            sys.stdout.write(data)
    except (OSError, ValueError, KeyError, configparser.Error, subprocess.CalledProcessError) as error:
        parser.exit(1, f"input inventory failed: {error}\n")


if __name__ == "__main__":
    main()
