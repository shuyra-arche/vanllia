"""Consistency checks use synthetic retained files, not claimed execution proof."""

import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from tests.support.paths import ROOT


class EvidenceGroup(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="evidence-group-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.cwd = str(self.base / "recorded-source")
        self.python = "/fixture/python3"
        self.commit, self.tree = "a" * 40, "b" * 40
        self.toolchains = [{"path": "rust-toolchain.toml", "toolchain": {"channel": "fixture-toolchain"}}]
        self.inventory = {
            "schema_version": 1, "kind": "committed-input-inventory",
            "is_release_manifest": False, "is_dependency_graph": False, "release_approved": False,
            "vanllia_commit": self.commit,
            "repositories": [
                {"path": ".", "commit": self.commit, "tree": self.tree, "toolchains": self.toolchains},
                {"path": "components/fixture", "commit": "c" * 40, "tree": "d" * 40},
            ],
        }
        self.inputs = self.base / "inputs.json"
        self.inputs.write_text(json.dumps(self.inventory, indent=2) + "\n")
        self.input_hash = self.digest(self.inputs)
        self.artifacts = self.base / "artifacts"
        self.artifacts.mkdir()
        for name in ("initrd", "kernel"):
            (self.artifacts / name).write_bytes(("synthetic " + name).encode())
        self.directories = {}
        self.records = {}
        for action in ("build", "python-tests", "rust-tests"):
            directory = self.base / action
            directory.mkdir()
            self.directories[action] = directory
            (directory / "inputs.json").write_bytes(self.inputs.read_bytes())
            sources = [{"path": x["path"], "commit": x["commit"], "clean": True} for x in self.inventory["repositories"]]
            config = {
                "input_inventory_sha256": self.input_hash, "configured_root_toolchains": self.toolchains,
                "profile": "release" if action == "build" else None,
                "environment": {"RUSTUP_TOOLCHAIN": "inherited" if action == "python-tests" else "fixture-toolchain", "RUSTFLAGS": "fixture-flags"},
            }
            if action != "python-tests":
                config["rustup_toolchain_override_from_snapshot"] = "fixture-toolchain"
            tools = [("git", ["git", "--version"]), ("python", [self.python, "--version"])]
            if action in ("build", "python-tests"):
                tools += [("nu", ["nu", "--version"])]
            if action in ("build", "rust-tests"):
                tools += [("rustup", ["rustup", "show", "active-toolchain"]), ("rustc", ["rustc", "-vV"]), ("cargo", ["cargo", "--version"])]
            probes = [self.observation(directory, "tool-" + name, argv,
                b"fixture-toolchain-x86_64-fixture\n" if name == "rustup" else ("synthetic " + name + "\n").encode()) for name, argv in tools]
            if action == "build":
                argv = ["nu", "scripts/build.nu", "release"]
                stdout = json.dumps({"scene": "product", "profile": "release", **{name: str(self.artifacts / name) for name in ("initrd", "kernel")}}).encode()
            elif action == "python-tests":
                argv = [self.python, "-B", "-m", "unittest", "discover", "-s", "tests", "-t", "."]
                stdout = b"synthetic retained test log\n"
            else:
                argv = ["cargo", "test", "--locked", "--offline", "-p", "vanllia-composition", "-p", "vanllia-image", "--tests", "--target", "x86_64-unknown-linux-gnu"]
                stdout = b"synthetic retained test log\n"
            execution = self.observation(directory, "execution", argv, stdout)
            execution["status"] = "exited"
            record = {
                "schema_version": 1, "kind": "execution-evidence", "synthetic_fixture": True,
                "is_release_manifest": False, "release_approved": False, "status": "observed", "missing": [],
                "action": action, "input": {"path": "inputs.json", "vanllia_commit": self.commit, "tree": self.tree, "inventory_sha256": self.input_hash, "verified": True},
                "source": {"status": "verified-before-and-after", "before": sources, "after": copy.deepcopy(sources)},
                "producer": {"sha256": "9" * 64}, "configuration": config, "tools": probes,
                "execution": execution, "artifacts": [],
                "inventory_replay": self.observation(directory, "inventory", [self.python, "-B", str(Path(self.cwd) / "docs/generate-composition-inputs.py"), "--root", self.cwd, "--commit", self.commit], self.inputs.read_bytes()),
            }
            if action == "build":
                plan = {"scene": "product", "profile": "release", "cwd": self.cwd, "product": ["cargo", "image", "build", "product", "release"], "kernel": ["cargo", "build", "fixture"]}
                config["build_plan"] = plan
                record["build_plan_observation"] = self.observation(directory, "build-plan", ["nu", "scripts/build.nu", "release", "--plan"], json.dumps(plan).encode())
                record["artifacts"] = [{"name": name, "status": "observed", "reported_path": str(self.artifacts / name), "resolved_path": str(self.artifacts / name), **self.fingerprint(self.artifacts / name)} for name in ("initrd", "kernel")]
            self.records[action] = record
            self.store(action)

    @staticmethod
    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def fingerprint(self, path):
        return {"bytes": path.stat().st_size, "sha256": self.digest(path)}

    def observation(self, directory, name, argv, stdout=b"", stderr=b""):
        value = {"argv": argv, "cwd": self.cwd, "returncode": 0}
        for stream, data in (("stdout", stdout), ("stderr", stderr)):
            path = directory / (name + "." + stream + ".log")
            path.write_bytes(data)
            value[stream] = {"path": path.name, **self.fingerprint(path)}
        return value

    def store(self, action):
        (self.directories[action] / "record.json").write_text(json.dumps(self.records[action], indent=2))

    def invoke(self, actions=None, profile="release"):
        actions = list(self.directories) if actions is None else actions
        argv = [sys.executable, "-B", str(ROOT / "scripts/check-evidence.py"), "--inputs", str(self.inputs), "--profile", profile]
        for action in actions:
            argv += ["--record", str(self.directories[action])]
        # A file-only checker must work without Git/Cargo/Nu on PATH.
        result = subprocess.run(argv, capture_output=True, text=True, env={**os.environ, "PATH": str(self.base / "no-tools")})
        return result, json.loads(result.stdout)

    def rejected(self, fragment, actions=None, profile="release"):
        result, report = self.invoke(actions, profile)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(report["status"], "inconsistent")
        self.assertFalse(report["release_approved"])
        self.assertIn(fragment, " ".join(x["message"] for x in report["errors"]))
        return report

    def test_complete_group_is_consistent_and_checker_does_not_write(self):
        before = {str(p): self.digest(p) for p in self.base.rglob("*") if p.is_file()}
        result, report = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(report["status"], "consistent")
        self.assertEqual(report["input"]["vanllia_commit"], self.commit)
        self.assertFalse(report["is_release_manifest"])
        self.assertFalse(report["release_approved"])
        self.assertIn("test counts and coverage", report["not_evaluated"])
        self.assertIn("guest runtime", report["not_evaluated"])
        self.assertEqual(before, {str(p): self.digest(p) for p in self.base.rglob("*") if p.is_file()})

    def test_record_order_and_python_inherited_rust_selection_do_not_change_group(self):
        result, report = self.invoke(["python-tests", "rust-tests", "build"])
        self.assertEqual(result.returncode, 0, report)

    def test_valid_symlink_artifact_keeps_its_resolved_hash_association(self):
        artifact = self.records["build"]["artifacts"][0]
        alias = self.artifacts / "initrd-alias"
        alias.symlink_to(self.artifacts / "initrd")
        artifact["reported_path"] = str(alias)
        observation = self.records["build"]["execution"]["stdout"]
        path = self.directories["build"] / observation["path"]
        result = json.loads(path.read_text())
        result["initrd"] = str(alias)
        path.write_text(json.dumps(result))
        observation.update(self.fingerprint(path))
        self.store("build")
        result, report = self.invoke()
        self.assertEqual(result.returncode, 0, report)

    def test_no_records_reports_each_required_action_missing(self):
        report = self.rejected("missing required action", [])
        self.assertEqual({x["record"] for x in report["errors"]}, set(self.directories))

    def test_missing_record_file_is_not_an_empty_success(self):
        (self.directories["build"] / "record.json").unlink()
        self.rejected("No such file")

    def test_missing_duplicate_and_unexpected_actions_are_rejected(self):
        self.rejected("missing required action", ["build", "python-tests"])
        self.rejected("duplicate required action", ["build", "build", "python-tests", "rust-tests"])
        self.records["build"]["action"] = "runtime"
        self.store("build")
        self.rejected("unexpected/missing action")

    def test_report_keeps_individual_cross_record_and_missing_action_errors(self):
        self.records["rust-tests"]["action"] = "unexpected"
        self.store("rust-tests")
        self.records["python-tests"]["configuration"]["environment"]["RUSTFLAGS"] = "different flags"
        self.store("python-tests")
        report = self.rejected("missing required action")
        errors = {(x["record"], x["message"]) for x in report["errors"]}
        self.assertIn((str(self.directories["rust-tests"] / "record.json"), "unexpected/missing action"), errors)
        self.assertIn((str(self.directories["python-tests"] / "record.json"), "recorded environment differs across actions"), errors)
        self.assertIn(("rust-tests", "missing required action"), errors)

    def test_different_commit_tree_or_snapshot_cannot_be_mixed(self):
        original = copy.deepcopy(self.records["rust-tests"])
        for key, value, message in (("vanllia_commit", "e" * 40, "commit/tree"), ("tree", "f" * 40, "commit/tree"), ("inventory_sha256", "0" * 64, "snapshot hash")):
            with self.subTest(key=key):
                self.records["rust-tests"] = copy.deepcopy(original)
                self.records["rust-tests"]["input"][key] = value
                self.store("rust-tests")
                self.rejected(message)

    def test_wrong_copied_input_and_replay_content_are_rejected(self):
        path = self.directories["python-tests"] / "inputs.json"
        path.write_text("{}")
        self.rejected("copied input snapshot")
        path.write_bytes(self.inputs.read_bytes())
        replay = self.records["python-tests"]["inventory_replay"]["stdout"]
        path = self.directories["python-tests"] / replay["path"]
        path.write_text("{}")
        replay.update(self.fingerprint(path))
        self.store("python-tests")
        self.rejected("inventory replay output")

    def test_incomplete_source_and_claimed_success_are_rejected(self):
        self.records["python-tests"]["source"]["after"].pop()
        self.store("python-tests")
        self.rejected("source before/after")

    def test_failed_missing_or_unrun_record_is_rejected(self):
        original = copy.deepcopy(self.records["python-tests"])
        for field, value in (("status", "failed"), ("missing", ["unavailable tool"]), ("release_approved", True)):
            with self.subTest(field=field):
                self.records["python-tests"] = copy.deepcopy(original)
                self.records["python-tests"][field] = value
                self.store("python-tests")
                self.rejected("record")
        self.records["python-tests"] = copy.deepcopy(original)
        self.records["python-tests"]["execution"]["status"] = "not-run"
        self.store("python-tests")
        self.rejected("not completed")

    def test_expected_profile_and_root_toolchain_configuration_are_checked(self):
        self.rejected("profile", profile="debug")
        self.records["rust-tests"]["configuration"]["configured_root_toolchains"] = []
        self.store("rust-tests")
        self.rejected("configured toolchains")

    def test_rust_selection_and_environment_differences_are_rejected(self):
        record = self.records["rust-tests"]
        record["configuration"]["rustup_toolchain_override_from_snapshot"] = "other"
        self.store("rust-tests")
        self.rejected("Rust selection")
        record["configuration"]["rustup_toolchain_override_from_snapshot"] = "fixture-toolchain"
        record["configuration"]["environment"]["RUSTFLAGS"] = "different flags"
        self.store("rust-tests")
        self.rejected("environment differs")

    def test_shared_tool_version_output_must_agree_even_with_valid_file_hash(self):
        probe = self.records["rust-tests"]["tools"][-1]
        path = self.directories["rust-tests"] / probe["stdout"]["path"]
        path.write_text("another cargo version\n")
        probe["stdout"].update(self.fingerprint(path))
        self.store("rust-tests")
        self.rejected("tool version output differs")

    def test_missing_duplicate_probe_and_changed_recorder_are_rejected(self):
        original = copy.deepcopy(self.records["python-tests"])
        self.records["python-tests"]["tools"].pop()
        self.store("python-tests")
        self.rejected("required tool probes")
        self.records["python-tests"] = copy.deepcopy(original)
        self.records["python-tests"]["tools"].append(copy.deepcopy(original["tools"][0]))
        self.store("python-tests")
        self.rejected("duplicate tool probe")
        self.records["python-tests"] = copy.deepcopy(original)
        self.records["python-tests"]["producer"]["sha256"] = "8" * 64
        self.store("python-tests")
        self.rejected("recorder fingerprint differs")

    def test_missing_mutated_and_escaping_log_paths_are_rejected(self):
        observation = self.records["rust-tests"]["execution"]
        path = self.directories["rust-tests"] / observation["stdout"]["path"]
        original = path.read_bytes()
        path.unlink()
        self.rejected("No such file")
        path.write_bytes(original + b"changed")
        self.rejected("fingerprint mismatch")
        path.write_bytes(original)
        observation["stdout"]["path"] = "../inputs.json"
        self.store("rust-tests")
        self.rejected("must be relative")

    def test_missing_mutated_and_duplicate_artifacts_are_rejected(self):
        path = self.artifacts / "kernel"
        original = path.read_bytes()
        path.unlink()
        self.rejected("No such file")
        path.write_bytes(original + b"changed")
        self.rejected("fingerprint mismatch")
        path.write_bytes(original)
        self.records["build"]["artifacts"][1]["name"] = "initrd"
        self.store("build")
        self.rejected("duplicate artifact names")

    def test_artifact_and_configuration_must_match_captured_producer_output(self):
        record = self.records["build"]
        original = copy.deepcopy(record)
        record["artifacts"][0]["reported_path"] = str(self.base / "other")
        self.store("build")
        self.rejected("path differs from build output")
        self.records["build"] = original
        self.records["build"]["configuration"]["build_plan"]["kernel"] = ["different"]
        self.store("build")
        self.rejected("build plan/configuration")

    def test_command_and_nonzero_or_boolean_returncodes_are_not_trusted(self):
        original = copy.deepcopy(self.records["rust-tests"])
        for code in (1, False, None):
            with self.subTest(code=code):
                self.records["rust-tests"] = copy.deepcopy(original)
                self.records["rust-tests"]["execution"]["returncode"] = code
                self.store("rust-tests")
                self.rejected("did not return zero")
        self.records["rust-tests"] = original
        self.records["rust-tests"]["execution"]["argv"] = ["true"]
        self.store("rust-tests")
        self.rejected("Rust test command mismatch")

    def test_malformed_snapshot_and_record_shapes_report_inconsistency(self):
        original = self.inputs.read_bytes()
        self.inputs.write_text("[]")
        self.rejected("expected JSON object")
        self.inputs.write_bytes(original)
        record_path = self.directories["python-tests"] / "record.json"
        for value in ("{", "[]", '{"action":"python-tests"}'):
            with self.subTest(value=value):
                record_path.write_text(value)
                self.rejected("")

    def test_ambiguous_duplicate_json_fields_are_rejected(self):
        original = self.inputs.read_bytes()
        value = self.inputs.read_text().replace('"vanllia_commit":', '"vanllia_commit": "' + "f" * 40 + '", "vanllia_commit":', 1)
        self.inputs.write_text(value)
        self.rejected("duplicate JSON field")
        self.inputs.write_bytes(original)
        path = self.directories["python-tests"] / "record.json"
        path.write_text(path.read_text().replace('"action":', '"action": "build", "action":', 1))
        self.rejected("duplicate JSON field")
