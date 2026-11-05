#!/usr/bin/env python3
"""Read-only consistency check of one snapshot and three retained run records."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat


ACTIONS = ("build", "python-tests", "rust-tests")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def object_json(data, label):
    def unique_object(pairs):
        value = {}
        for key, item in pairs:
            require(key not in value, f"{label}: duplicate JSON field: {key}")
            value[key] = item
        return value

    value = json.loads(data, object_pairs_hook=unique_object)
    require(isinstance(value, dict), f"{label}: expected JSON object")
    return value


def fingerprint(path):
    require(stat.S_ISREG(path.stat().st_mode), f"not a regular file: {path}")
    with path.open("rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode), f"not a regular file: {path}")
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
        after = os.fstat(stream.fileno())
    fields = ("st_dev", "st_ino", "st_size", "st_mtime_ns")
    require(all(getattr(before, key) == getattr(after, key) for key in fields), f"file changed while hashing: {path}")
    return {"bytes": after.st_size, "sha256": digest}


def local_file(directory, name):
    require(isinstance(name, str) and bool(name), "missing evidence file path")
    path = Path(name)
    require(not path.is_absolute() and ".." not in path.parts, f"evidence path must be relative: {name}")
    path = (directory / path).resolve()
    require(path.is_relative_to(directory.resolve()), f"evidence path escapes directory: {name}")
    return path


def check_file(path, expected):
    require(isinstance(expected, dict), f"missing fingerprint: {path}")
    require(type(expected.get("bytes")) is int and expected["bytes"] >= 0, f"missing byte size: {path}")
    require(isinstance(expected.get("sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", expected["sha256"]), f"invalid SHA-256: {path}")
    actual = fingerprint(path)
    require(actual == {key: expected[key] for key in ("bytes", "sha256")}, f"fingerprint mismatch: {path}")


def check_observation(directory, observation, cwd):
    require(isinstance(observation, dict), "missing process observation")
    require(type(observation.get("returncode")) is int and observation["returncode"] == 0, "process did not return zero")
    require(observation.get("cwd") == cwd, "process cwd differs from execution cwd")
    argv = observation.get("argv")
    require(isinstance(argv, list) and argv and all(isinstance(x, str) and x for x in argv), "missing process argv")
    files = {}
    for stream in ("stdout", "stderr"):
        item = observation.get(stream)
        require(isinstance(item, dict), f"missing {stream} log")
        files[stream] = local_file(directory, item.get("path"))
        check_file(files[stream], item)
    return files


def expected_input(path):
    data = path.read_bytes()
    inventory = object_json(data, "input snapshot")
    require(type(inventory.get("schema_version")) is int and inventory["schema_version"] == 1, "unsupported input schema")
    require(inventory.get("kind") == "committed-input-inventory", "wrong input kind")
    for key in ("is_release_manifest", "is_dependency_graph", "release_approved"):
        require(inventory.get(key) is False, f"input {key} must be false")
    commit = inventory.get("vanllia_commit")
    require(isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit), "missing full input commit")
    repositories = inventory.get("repositories")
    require(isinstance(repositories, list) and repositories, "missing input repositories")
    require(all(isinstance(x, dict) for x in repositories), "invalid input repository")
    for repository in repositories:
        path = repository.get("path")
        require(isinstance(path, str) and path and not Path(path).is_absolute() and ".." not in Path(path).parts, "invalid source path")
        for key in ("commit", "tree"):
            require(isinstance(repository.get(key), str) and re.fullmatch(r"[0-9a-f]{40}", repository[key]), f"invalid source {key}")
    paths = [x.get("path") for x in repositories]
    require(all(isinstance(x, str) for x in paths) and len(set(paths)) == len(paths), "duplicate/invalid input repository paths")
    roots = [x for x in repositories if x.get("path") == "."]
    require(len(roots) == 1 and roots[0].get("commit") == commit, "input root commit mismatch")
    root = roots[0]
    require(isinstance(root.get("tree"), str) and re.fullmatch(r"[0-9a-f]{40}", root["tree"]), "missing input root tree")
    toolchains = root.get("toolchains")
    require(isinstance(toolchains, list), "missing configured root toolchains")
    selections = [x for x in toolchains if isinstance(x, dict) and x.get("path") == "rust-toolchain.toml"]
    require(len(selections) == 1 and isinstance(selections[0].get("toolchain"), dict), "missing configured Rust toolchain")
    channel = selections[0]["toolchain"].get("channel")
    require(isinstance(channel, str) and channel, "missing configured Rust channel")
    sources = [{"path": x["path"], "commit": x["commit"], "clean": True} for x in repositories]
    return inventory, hashlib.sha256(data).hexdigest(), root, channel, sources


def check_record(directory, record, inventory, input_hash, root, channel, sources, profile):
    require(type(record.get("schema_version")) is int and record["schema_version"] == 1, "unsupported record schema")
    require(record.get("kind") == "execution-evidence", "wrong record kind")
    require(record.get("is_release_manifest") is False and record.get("release_approved") is False, "record must not claim release approval")
    require(record.get("status") == "observed" and record.get("missing") == [], "record is failed/incomplete or has missing evidence")
    action = record["action"]
    binding = record["input"]
    require(binding.get("verified") is True, "input replay was not verified")
    require(binding.get("vanllia_commit") == inventory["vanllia_commit"] and binding.get("tree") == root["tree"], "input commit/tree mismatch")
    require(binding.get("inventory_sha256") == input_hash, "input snapshot hash mismatch")
    copied = local_file(directory, binding.get("path")).read_bytes()
    require(hashlib.sha256(copied).hexdigest() == input_hash, "copied input snapshot hash mismatch")
    source = record["source"]
    require(source.get("status") == "verified-before-and-after" and source.get("before") == sources and source.get("after") == sources, "source before/after does not match snapshot")
    require(all(x.get("clean") is True for phase in ("before", "after") for x in source[phase]), "source clean status must be true")
    producer = record["producer"].get("sha256")
    require(isinstance(producer, str) and re.fullmatch(r"[0-9a-f]{64}", producer), "missing recorder fingerprint")
    config = record["configuration"]
    require(config.get("input_inventory_sha256") == input_hash, "configuration input hash mismatch")
    require(config.get("configured_root_toolchains") == root["toolchains"], "configured toolchains differ from snapshot")
    require(config.get("profile") == (profile if action == "build" else None), "unexpected build/test profile")
    environment = config.get("environment")
    require(isinstance(environment, dict) and all(isinstance(k, str) and isinstance(v, str) for k, v in environment.items()), "missing/invalid recorded environment")
    if action != "python-tests":
        require(config.get("rustup_toolchain_override_from_snapshot") == channel and environment.get("RUSTUP_TOOLCHAIN") == channel, "Rust selection differs from snapshot")
    execution = record["execution"]
    cwd = execution.get("cwd")
    require(isinstance(cwd, str) and Path(cwd).is_absolute(), "missing absolute execution cwd")
    require(execution.get("status") == "exited", "execution was not completed")
    execution_files = check_observation(directory, execution, cwd)
    tools = record.get("tools")
    require(isinstance(tools, list), "missing tool observations")
    probes = {}
    version_signatures = {}
    for tool in tools:
        files = check_observation(directory, tool, cwd)
        argv = tool["argv"]
        known = {
            ("git", "--version"): "git", ("nu", "--version"): "nu",
            ("rustup", "show", "active-toolchain"): "rustup",
            ("rustc", "-vV"): "rustc", ("cargo", "--version"): "cargo",
        }
        name = known.get(tuple(argv))
        if name is None and len(argv) == 2 and argv[1] == "--version" and Path(argv[0]).is_absolute():
            name = "python"
        require(name is not None and name not in probes, "unexpected/duplicate tool probe")
        probes[name] = tool
        version_signatures[name] = [tool[x]["sha256"] for x in ("stdout", "stderr")]
        if name == "rustup":
            active = files["stdout"].read_text().strip()
            require(active == channel or active.startswith(channel + "-") or active.startswith(channel + " "), "observed Rust toolchain mismatch")
    required = {"git", "python"}
    if action in ("build", "python-tests"):
        required.add("nu")
    if action in ("build", "rust-tests"):
        required.update(("rustup", "rustc", "cargo"))
    require(set(probes) == required, "missing/unexpected required tool probes")
    python = probes["python"]["argv"][0]
    replay = record["inventory_replay"]
    replay_files = check_observation(directory, replay, cwd)
    require(replay["argv"] == [python, "-B", str(Path(cwd) / "docs/generate-composition-inputs.py"), "--root", cwd, "--commit", inventory["vanllia_commit"]], "inventory replay command mismatch")
    require(object_json(replay_files["stdout"].read_bytes(), "inventory replay") == inventory, "inventory replay output mismatch")
    if action == "build":
        require(execution["argv"] == ["nu", "scripts/build.nu", profile], "build command mismatch")
        plan_observation = record["build_plan_observation"]
        plan_files = check_observation(directory, plan_observation, cwd)
        require(plan_observation["argv"] == ["nu", "scripts/build.nu", profile, "--plan"], "build plan command mismatch")
        plan = object_json(plan_files["stdout"].read_bytes(), "build plan")
        require(config.get("build_plan") == plan and plan.get("scene") == "product" and plan.get("profile") == profile, "build plan/configuration mismatch")
        require(isinstance(plan.get("cwd"), str) and Path(plan["cwd"]).is_absolute(), "missing build plan cwd")
        require(all(isinstance(plan.get(key), list) and plan[key] and all(isinstance(x, str) and x for x in plan[key]) for key in ("product", "kernel")), "missing build plan commands")
        result = object_json(execution_files["stdout"].read_bytes(), "build result")
        require(result.get("scene") == "product" and result.get("profile") == profile, "build result profile mismatch")
        artifacts = record.get("artifacts")
        require(isinstance(artifacts, list) and len(artifacts) == 2 and all(isinstance(x, dict) for x in artifacts), "missing build artifacts")
        require({x.get("name") for x in artifacts} == {"initrd", "kernel"}, "missing/duplicate artifact names")
        for artifact in artifacts:
            require(artifact.get("status") == "observed", "artifact was not observed")
            require(artifact.get("reported_path") == result.get(artifact["name"]), "artifact path differs from build output")
            reported = artifact.get("reported_path")
            require(isinstance(reported, str) and Path(reported).is_absolute(), "missing reported artifact path")
            path = artifact.get("resolved_path")
            require(isinstance(path, str) and Path(path).is_absolute(), "missing absolute artifact path")
            require(Path(reported).resolve() == Path(path).resolve(), "reported/resolved artifact paths no longer agree")
            check_file(Path(path), artifact)
    elif action == "python-tests":
        require(execution["argv"] == [python, "-B", "-m", "unittest", "discover", "-s", "tests", "-t", "."], "Python test command mismatch")
        require(record.get("artifacts") == [], "unexpected test artifacts")
    else:
        require(execution["argv"] == ["cargo", "test", "--locked", "--offline", "-p", "vanllia-composition", "-p", "vanllia-image", "--tests", "--target", "x86_64-unknown-linux-gnu"], "Rust test command mismatch")
        require(record.get("artifacts") == [], "unexpected test artifacts")
    return environment, version_signatures, producer


def check_group(inputs, directories, profile):
    report = {
        "schema_version": 1, "kind": "evidence-group-consistency",
        "status": "inconsistent", "is_release_manifest": False, "release_approved": False,
        "input": None, "required_actions": list(ACTIONS), "expected_build_profile": profile,
        "records": [], "errors": [],
        "not_evaluated": ["execution authenticity/signatures", "test counts and coverage", "guest runtime", "dependency graphs", "artifact freshness/retention", "complete environment/reproducibility", "firmware", "version/release approval"],
    }
    try:
        inventory, input_hash, root, channel, sources = expected_input(inputs)
        report["input"] = {"vanllia_commit": inventory["vanllia_commit"], "tree": root["tree"], "inventory_sha256": input_hash}
    except (OSError, ValueError, KeyError, TypeError) as error:
        report["errors"].append({"record": "input snapshot", "message": str(error)})
        return report
    counts = dict.fromkeys(ACTIONS, 0)
    valid = []
    for directory in directories:
        entry = {"path": str(directory / "record.json"), "action": None, "status": "inconsistent"}
        report["records"].append(entry)
        try:
            raw = (directory / "record.json").read_bytes()
            record = object_json(raw, "record")
            entry["sha256"] = hashlib.sha256(raw).hexdigest()
            action = record.get("action")
            require(isinstance(action, str) and action in counts, "unexpected/missing action")
            entry["action"] = action
            counts[action] += 1
            environment, versions, producer = check_record(directory, record, inventory, input_hash, root, channel, sources, profile)
            valid.append((entry, environment, versions, producer))
            entry["status"] = "consistent"
        except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
            report["errors"].append({"record": entry["path"], "message": str(error)})
    for action, count in counts.items():
        if count != 1:
            report["errors"].append({"record": action, "message": "missing required action" if count == 0 else "duplicate required action"})
    if valid:
        _, baseline_environment, baseline_versions, baseline_producer = valid[0]
        common_versions = dict(baseline_versions)
        for entry, environment, versions, producer in valid[1:]:
            if {k: v for k, v in environment.items() if k != "RUSTUP_TOOLCHAIN"} != {k: v for k, v in baseline_environment.items() if k != "RUSTUP_TOOLCHAIN"}:
                report["errors"].append({"record": entry["path"], "message": "recorded environment differs across actions"})
            if producer != baseline_producer:
                report["errors"].append({"record": entry["path"], "message": "recorder fingerprint differs across actions"})
            for name, signature in versions.items():
                if name in common_versions and common_versions[name] != signature:
                    report["errors"].append({"record": entry["path"], "message": f"tool version output differs across actions: {name}"})
                common_versions[name] = signature
    if not report["errors"]:
        report["status"] = "consistent"
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--record", type=Path, action="append", default=[], help="retained record directory; repeat for all three actions")
    parser.add_argument("--profile", choices=("debug", "release"), default="release", help="required build profile")
    args = parser.parse_args()
    report = check_group(args.inputs, args.record, args.profile)
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report["status"] == "consistent" else 1)


if __name__ == "__main__":
    main()
