#!/usr/bin/env python3
"""Observe one existing build/host-test entry against a verified input snapshot."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone


ENVIRONMENT_KEYS = (
    "RUSTUP_TOOLCHAIN", "RUSTUP_HOME", "CARGO_HOME", "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR",
    "CARGO_BUILD_JOBS", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
)


def git_bytes(root, *args):
    return subprocess.check_output([
        "git", "--no-lazy-fetch", "--no-replace-objects", "--no-optional-locks",
        "-C", str(root), *args,
    ])


def git(root, *args):
    return git_bytes(root, *args).decode().strip()


def file_record(path):
    path = Path(path)
    before = path.stat()
    if not path.is_file():
        raise ValueError(f"not a regular file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    after = path.stat()
    if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) != (
        after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns
    ):
        raise ValueError(f"file changed while hashing: {path}")
    return {"bytes": after.st_size, "sha256": digest.hexdigest()}


def checkout_dirty(path, historical=()):
    # Historical worktree content is outside the required input set. Its staged
    # gitlink is still part of the root source commit and may not change.
    if historical and git(path, "diff", "--cached", "--name-only", "--ignore-submodules=none", "HEAD", "--", *historical):
        return "staged historical gitlink change"
    exclusions = [":(top,exclude,literal)" + name for name in historical]
    return git(path, "status", "--porcelain", "--untracked-files=all", "--ignore-submodules=none", "--", ".", *exclusions)


def historical_observation(root, inventory):
    return [{
        "path": reference["path"], "commit": reference["commit"],
        "checkout": "present-unverified" if os.path.lexists(root / reference["path"]) else "absent",
        "content_verified": False,
    } for reference in inventory["historical_references"]]


def source_check(root, inventory):
    observations = []
    historical = [item["path"] for item in inventory["historical_references"]]
    for repository in inventory["repositories"]:
        path = root / repository["path"]
        if Path(git(path, "rev-parse", "--show-toplevel")).resolve() != path.resolve():
            raise ValueError(f"missing source checkout: {repository['path']}")
        head = git(path, "rev-parse", "HEAD")
        dirty = checkout_dirty(path, historical if repository["path"] == "." else ())
        if head != repository["commit"] or dirty:
            raise ValueError(f"source does not match clean snapshot checkout: {repository['path']}")
        observations.append({"path": repository["path"], "commit": head, "clean": True})
    return observations


def observe(args, output):
    root = args.root.resolve()
    producer_bytes = Path(__file__).read_bytes()
    record = {
        "schema_version": 2, "kind": "execution-evidence",
        "is_release_manifest": False, "release_approved": False,
        "status": "incomplete", "input": None, "source": {"status": "unverified"},
        "producer": {"sha256": hashlib.sha256(producer_bytes).hexdigest()},
        "action": args.action, "configuration": {}, "tools": [],
        "execution": {"status": "not-run", "returncode": None},
        "historical": {"status": "reference-only-not-verified", "before": [], "after": []},
        "artifacts": [], "missing": [],
        "not_evaluated": [
            "dependency graphs", "test case counts and coverage", "artifact freshness",
            "hermetic environment and reproducibility", "firmware", "guest runtime",
            "version decisions and release approval",
        ],
    }
    environment = os.environ.copy()
    inventory = None

    def run(name, argv):
        observation = {"argv": argv, "cwd": str(root), "returncode": None}
        for channel in ("stdout", "stderr"):
            observation[channel] = {"path": name + "." + channel + ".log"}
        started = time.monotonic()
        observation["started_utc"] = datetime.now(timezone.utc).isoformat()
        with (output / observation["stdout"]["path"]).open("wb") as stdout, (
            output / observation["stderr"]["path"]
        ).open("wb") as stderr:
            try:
                observation["returncode"] = subprocess.run(
                    argv, cwd=root, env=environment, stdout=stdout, stderr=stderr,
                ).returncode
            finally:
                observation["finished_utc"] = datetime.now(timezone.utc).isoformat()
                observation["elapsed_seconds"] = time.monotonic() - started
        for channel in ("stdout", "stderr"):
            observation[channel].update(file_record(output / observation[channel]["path"]))
        return observation

    try:
        snapshot_bytes = args.inputs.read_bytes()
        (output / "inputs.json").write_bytes(snapshot_bytes)
        inventory = json.loads(snapshot_bytes)
        if not isinstance(inventory, dict) or type(inventory.get("schema_version")) is not int or inventory["schema_version"] != 2:
            raise ValueError("recorder requires input schema 2; retained schema-1 groups keep their original meaning")
        commit = inventory["vanllia_commit"]
        if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
            raise ValueError("snapshot has no full input commit")
        record["input"] = {
            "path": "inputs.json",
            "vanllia_commit": commit,
            "inventory_sha256": hashlib.sha256(snapshot_bytes).hexdigest(),
            "verified": False,
        }
        historical = [item["path"] for item in inventory["historical_references"]]
        if git(root, "rev-parse", "HEAD") != commit or checkout_dirty(root, historical):
            raise ValueError("root must be a clean checkout at the snapshot commit")
        if git_bytes(root, "show", commit + ":scripts/record-evidence.py") != producer_bytes:
            raise ValueError("recorder bytes differ from the input commit")
        replay = run("inventory", [
            sys.executable, "-B", str(root / "docs/generate-composition-inputs.py"),
            "--root", str(root), "--commit", commit,
        ])
        record["inventory_replay"] = replay
        if replay["returncode"] != 0 or json.loads(
            (output / replay["stdout"]["path"]).read_bytes()
        ) != inventory:
            raise ValueError("input snapshot does not match regenerated committed inputs")
        record["source"]["before"] = source_check(root, inventory)
        record["historical"]["before"] = historical_observation(root, inventory)
        record["input"].update({"verified": True, "tree": git(root, "rev-parse", commit + "^{tree}")})
        configuration = record["configuration"]
        configuration["input_inventory_sha256"] = record["input"]["inventory_sha256"]
        root_repository = next(item for item in inventory["repositories"] if item["path"] == ".")
        configuration["configured_root_toolchains"] = root_repository["toolchains"]
        configuration["environment_scope"] = "selected overrides only; external/global config is not exhaustively captured"
        rust_action = args.action in {"build", "rust-tests"}
        if rust_action:
            toolchain = next((x for x in root_repository["toolchains"] if x["path"] == "rust-toolchain.toml"), None)
            if not toolchain or not isinstance(toolchain.get("toolchain"), dict):
                raise ValueError("snapshot is missing the configured root Rust toolchain")
            channel = toolchain["toolchain"].get("channel")
            if not isinstance(channel, str) or not channel:
                raise ValueError("snapshot is missing the root Rust toolchain channel")
            environment["RUSTUP_TOOLCHAIN"] = channel
            configuration["rustup_toolchain_override_from_snapshot"] = channel
        configuration["environment"] = {k: environment[k] for k in ENVIRONMENT_KEYS if k in environment}
        configuration["profile"] = args.profile if args.action == "build" else None
        probes = [["git", "--version"], [sys.executable, "--version"]]
        if args.action in {"build", "python-tests"}:
            probes.append(["nu", "--version"])
        if rust_action:
            probes += [["rustup", "show", "active-toolchain"], ["rustc", "-vV"], ["cargo", "--version"]]
        for index, argv in enumerate(probes):
            resolved = shutil.which(argv[0])
            if resolved is None:
                raise ValueError(f"missing tool: {argv[0]}")
            probe = run(f"tool-{index}", argv)
            probe["resolved_executable"] = resolved
            record["tools"].append(probe)
            if probe["returncode"] != 0:
                raise ValueError(f"tool version probe failed: {argv[0]}")
            if argv[:2] == ["rustup", "show"]:
                active = (output / probe["stdout"]["path"]).read_text().strip()
                if not (active.startswith(channel + "-") or active.startswith(channel + " ") or active == channel):
                    raise ValueError("active Rust toolchain differs from the configured selection")
        if args.action == "build":
            plan = run("build-plan", ["nu", "scripts/build.nu", args.profile, "--plan"])
            record["build_plan_observation"] = plan
            if plan["returncode"] != 0:
                raise ValueError("build entry plan failed")
            build_plan = json.loads((output / plan["stdout"]["path"]).read_bytes())
            if not isinstance(build_plan, dict) or any(
                build_plan.get(key) != value
                for key, value in {"scene": "product", "profile": args.profile}.items()
            ) or not isinstance(build_plan.get("cwd"), str) or (
                Path(build_plan["cwd"]).resolve() != root
            ) or any(
                not isinstance(build_plan.get(key), list) or not build_plan[key]
                or not all(isinstance(value, str) for value in build_plan[key])
                for key in ("product", "kernel")
            ):
                raise ValueError("invalid build entry plan")
            configuration["build_plan"] = build_plan
            argv = ["nu", "scripts/build.nu", args.profile]
        elif args.action == "python-tests":
            argv = [sys.executable, "-B", "-m", "unittest", "discover", "-s", "tests", "-t", "."]
        else:
            argv = ["cargo", "test", "--locked", "--offline", "-p", "vanllia-composition", "-p", "vanllia-image", "--tests", "--target", "x86_64-unknown-linux-gnu"]
        execution = run("execution", argv)
        execution["status"] = "exited"
        record["execution"] = execution
        if args.action == "build" and execution["returncode"] == 0:
            result = json.loads((output / execution["stdout"]["path"]).read_bytes())
            if not isinstance(result, dict) or result.get("scene") != "product" or result.get("profile") != args.profile:
                raise ValueError("build result scene/profile mismatch")
            for name in ("initrd", "kernel"):
                artifact = {"name": name, "status": "missing", "origin": "path reported by build entry; freshness not established"}
                record["artifacts"].append(artifact)
                try:
                    path = Path(result[name])
                    if not path.is_absolute():
                        raise ValueError("artifact path must be absolute")
                    resolved = path.resolve()
                    artifact.update({"reported_path": str(path), "resolved_path": str(resolved), **file_record(resolved), "status": "observed"})
                except (OSError, ValueError, KeyError, TypeError) as error:
                    record["missing"].append(f"{name}: {error}")
    except (OSError, ValueError, KeyError, TypeError, StopIteration, subprocess.CalledProcessError) as error:
        record["missing"].append(str(error))
    finally:
        if record["input"] and record["input"]["verified"]:
            try:
                record["source"]["after"] = source_check(root, inventory)
                record["historical"]["after"] = historical_observation(root, inventory)
                if record["source"].get("before") != record["source"]["after"]:
                    raise ValueError("source checks before and after differ")
                record["source"]["status"] = "verified-before-and-after"
            except (OSError, ValueError, subprocess.CalledProcessError) as error:
                record["missing"].append(f"post-run source check: {error}")
        returncode = record["execution"]["returncode"]
        record["status"] = "failed" if returncode not in (None, 0) else (
            "observed" if returncode == 0 and not record["missing"] else "incomplete"
        )
        (output / "record.json").write_text(json.dumps(record, indent=2) + "\n")
    return 0 if record["status"] == "observed" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="new evidence directory outside the source checkout")
    parser.add_argument("--action", choices=("build", "python-tests", "rust-tests"), required=True)
    parser.add_argument("--profile", choices=("debug", "release"))
    args = parser.parse_args()
    if args.action != "build" and args.profile is not None:
        parser.error("--profile applies only to build")
    args.profile = args.profile or "release"
    output = args.output.resolve()
    if output.is_relative_to(args.root.resolve()):
        parser.error("evidence output must be outside the source checkout")
    try:
        output.mkdir()
    except OSError as error:
        parser.error(str(error))
    raise SystemExit(observe(args, output))


if __name__ == "__main__":
    main()
