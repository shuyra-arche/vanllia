"""Execution-record contracts; tiny Git sources and tool doubles are synthetic."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from tests.support.paths import ROOT


NU_DOUBLE = '''#!/usr/bin/env python3
import json, os, pathlib, subprocess, sys
mode = os.environ.get("EVIDENCE_FIXTURE_MODE", "")
if "--version" in sys.argv:
    print("synthetic-nu")
    raise SystemExit(6 if mode == "probe-failure" else 0)
root = pathlib.Path.cwd()
profile = sys.argv[2]
if "--plan" in sys.argv:
    plan = {"scene": "product", "profile": profile, "cwd": str(root),
            "product": ["cargo", "image", "build", "product", profile],
            "kernel": ["cargo", "build", "synthetic-kernel"]}
    if mode == "symlink-plan": plan["cwd"] = str(root.parent / "source-alias")
    print(json.dumps([] if mode == "invalid-plan" else plan))
    raise SystemExit(0)
print("synthetic build only", file=sys.stderr)
if mode == "build-failure": raise SystemExit(7)
if mode == "invalid-result":
    print("[]")
    raise SystemExit(0)
target = root / "target"
target.mkdir(exist_ok=True)
initrd, kernel = target / "discovered.img", target / "discovered.elf"
initrd.write_bytes(b"synthetic-initrd")
kernel.write_bytes(b"synthetic-kernel")
if mode == "symlink-artifact":
    payload = target / "resolved-kernel"
    kernel.rename(payload)
    kernel.symlink_to(payload)
if mode == "missing-artifact": kernel.unlink()
if mode == "directory-artifact":
    kernel.unlink()
    kernel.mkdir()
if mode == "mutate-source": (root / "tracked.txt").write_text("changed during command")
if mode == "mutate-history":
    history = root / "legacy/sqware"
    history.mkdir(parents=True)
    (history / "unverified.txt").write_text("not a build input")
if mode == "mutate-history-index":
    subprocess.run(["git", "update-index", "--cacheinfo", "160000," + "f" * 40 + ",legacy/sqware"], check=True)
print(json.dumps({"scene": "product", "profile": profile,
                  "initrd": str(initrd), "kernel": str(kernel)}))
'''

RUST_DOUBLE = '''#!/usr/bin/env python3
import os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
if name == "rustup":
    print("unexpected-toolchain" if os.environ.get("EVIDENCE_FIXTURE_MODE") == "wrong-toolchain"
          else os.environ["RUSTUP_TOOLCHAIN"] + "-x86_64-fixture")
else: print("synthetic " + name + " " + " ".join(sys.argv[1:]))
'''


class EvidenceRecord(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="evidence-contract-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.root = self.base / "source"
        self.root.mkdir()
        self.tools = self.base / "tools"
        self.tools.mkdir()
        self.env = os.environ.copy()
        self.env["PATH"] = str(self.tools) + os.pathsep + self.env["PATH"]
        self.env.pop("EVIDENCE_FIXTURE_MODE", None)
        self.env["RUSTUP_TOOLCHAIN"] = "different-inherited-selection"
        for name, body in [("nu", NU_DOUBLE), *[(n, RUST_DOUBLE) for n in ("cargo", "rustc", "rustup")]]:
            path = self.tools / name
            path.write_text(body)
            path.chmod(0o755)
        for directory in ("scripts", "docs", "tests"):
            (self.root / directory).mkdir()
        for path in ("scripts/record-evidence.py", "docs/generate-composition-inputs.py", "scripts/build.nu"):
            (self.root / path).write_bytes((ROOT / path).read_bytes())
        (self.root / "Cargo.toml").write_text('[workspace]\n')
        (self.root / "Cargo.lock").write_text('version = 4\n')
        (self.root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "fixture-toolchain"\n')
        (self.root / ".gitignore").write_text('/target/\n__pycache__/\n')
        (self.root / "tracked.txt").write_text("original")
        (self.root / "tests/__init__.py").write_text("")
        (self.root / "tests/test_sample.py").write_text(
            "import unittest\nclass Sample(unittest.TestCase):\n"
            "    def test_observed(self): self.assertTrue(True)\n"
        )
        self.git("init", "--quiet")
        self.git("config", "user.name", "Local evidence fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.inputs = self.base / "inputs.json"
        self.freeze()
        self.output = self.base / "record"

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], text=True).strip()

    def freeze(self, gitlinks=()):
        self.git("add", ".")
        for path, commit in gitlinks:
            self.git("update-index", "--add", "--cacheinfo", f"160000,{commit},{path}")
        self.git("commit", "--quiet", "-m", "Synthetic input snapshot")
        self.commit = self.git("rev-parse", "HEAD")
        result = subprocess.run([
            sys.executable, "-B", str(ROOT / "docs/generate-composition-inputs.py"),
            "--root", str(self.root), "--commit", self.commit,
        ], capture_output=True, check=True)
        self.inputs.write_bytes(result.stdout)

    def invoke(self, action="build", *extra):
        return subprocess.run([
            sys.executable, "-B", str(ROOT / "scripts/record-evidence.py"),
            "--root", str(self.root), "--inputs", str(self.inputs),
            "--output", str(self.output), "--action", action, *extra,
        ], env=self.env, capture_output=True, text=True)

    def record(self):
        return json.loads((self.output / "record.json").read_text())

    def historical_reference(self):
        # The gitlink deliberately names an object unavailable in this checkout.
        self.historical_commit = "d" * 40
        (self.root / ".gitmodules").write_text(
            '[submodule "legacy/sqware"]\npath = legacy/sqware\n'
            'url = https://github.com/anreonyr/sqware.git\n')
        self.freeze([("legacy/sqware", self.historical_commit)])

    def required_provider(self):
        child = self.root / "components/provider"
        child.mkdir(parents=True)
        subprocess.run(["git", "init", "--quiet", str(child)], check=True)
        (child / "Cargo.toml").write_text("[workspace]\n")
        (child / "tracked.txt").write_text("provider source")
        subprocess.run(["git", "-C", str(child), "add", "."], check=True)
        subprocess.run(["git", "-C", str(child), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "Provider fixture"], check=True)
        commit = subprocess.check_output(["git", "-C", str(child), "rev-parse", "HEAD"], text=True).strip()
        with (self.root / ".gitmodules").open("a") as stream:
            stream.write('[submodule "provider"]\npath = components/provider\nurl = https://example.invalid/provider.git\n')
        self.freeze([("legacy/sqware", self.historical_commit), ("components/provider", commit)])
        return child

    def test_snapshot_is_deterministic_without_historical_checkout_or_objects(self):
        self.historical_reference()
        original = self.inputs.read_bytes()
        snapshot = json.loads(original)
        self.assertEqual(snapshot["schema_version"], 2)
        self.assertEqual([item["path"] for item in snapshot["repositories"]], ["."])
        reference = snapshot["historical_references"][0]
        self.assertEqual(reference["commit"], self.historical_commit)
        self.assertFalse(reference["checkout_required"])
        self.assertEqual(reference["content_status"], "not-inventoried")
        self.assertFalse((self.root / "legacy/sqware").exists())
        historical = self.root / "legacy/sqware"
        historical.mkdir(parents=True)
        (historical / "not-a-checkout.txt").write_text("optional and unverified")
        result = subprocess.run([sys.executable, "-B", str(ROOT / "docs/generate-composition-inputs.py"),
            "--root", str(self.root), "--commit", self.commit], capture_output=True, check=True)
        self.assertEqual(result.stdout, original)

    def test_missing_history_is_recorded_explicitly_without_blocking_execution(self):
        self.historical_reference()
        self.assertEqual(self.invoke().returncode, 0)
        record = self.record()
        self.assertEqual(record["schema_version"], 2)
        self.assertEqual(record["source"]["before"], [{"path": ".", "commit": self.commit, "clean": True}])
        self.assertEqual(record["historical"]["status"], "reference-only-not-verified")
        self.assertEqual(record["historical"]["before"], [{"path": "legacy/sqware",
            "commit": self.historical_commit, "checkout": "absent", "content_verified": False}])
        self.assertEqual(record["historical"]["before"], record["historical"]["after"])

    def test_history_presence_changes_are_observations_not_changed_inputs(self):
        self.historical_reference()
        self.env["EVIDENCE_FIXTURE_MODE"] = "mutate-history"
        self.assertEqual(self.invoke().returncode, 0)
        record = self.record()
        self.assertEqual(record["historical"]["before"][0]["checkout"], "absent")
        self.assertEqual(record["historical"]["after"][0]["checkout"], "present-unverified")
        self.assertFalse(record["historical"]["after"][0]["content_verified"])
        self.assertEqual(record["input"]["inventory_sha256"], hashlib.sha256(self.inputs.read_bytes()).hexdigest())

    def test_staged_historical_gitlink_change_still_prevents_execution(self):
        self.historical_reference()
        self.git("update-index", "--cacheinfo", "160000," + "e" * 40 + ",legacy/sqware")
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_historical_gitlink_change_during_command_breaks_source_binding(self):
        self.historical_reference()
        self.env["EVIDENCE_FIXTURE_MODE"] = "mutate-history-index"
        self.assertEqual(self.invoke().returncode, 1)
        record = self.record()
        self.assertEqual(record["execution"]["returncode"], 0)
        self.assertEqual(record["status"], "incomplete")
        self.assertIn("post-run source check", " ".join(record["missing"]))

    def test_history_exclusion_does_not_hide_real_untracked_or_config_changes(self):
        self.historical_reference()
        for relative in ("ordinary.rs", "legacy/sqware-not-history.txt", ".gitmodules"):
            with self.subTest(path=relative):
                path = self.root / relative
                original = path.read_bytes() if path.exists() else None
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("changed input")
                self.output = self.base / relative.replace("/", "-")
                self.assertEqual(self.invoke().returncode, 1)
                self.assertEqual(self.record()["execution"]["status"], "not-run")
                if original is None:
                    path.unlink()
                else:
                    path.write_bytes(original)

    def test_missing_required_provider_still_prevents_snapshot_and_execution(self):
        import shutil
        self.historical_reference()
        child = self.required_provider()
        shutil.rmtree(child)
        generated = subprocess.run([sys.executable, "-B", str(ROOT / "docs/generate-composition-inputs.py"),
            "--root", str(self.root), "--commit", self.commit], capture_output=True, text=True)
        self.assertNotEqual(generated.returncode, 0)
        self.assertIn("components/provider", generated.stderr)
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_dirty_or_wrong_required_provider_is_not_optional_with_missing_history(self):
        self.historical_reference()
        child = self.required_provider()
        (child / "tracked.txt").write_text("dirty provider")
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")
        subprocess.run(["git", "-C", str(child), "add", "."], check=True)
        subprocess.run(["git", "-C", str(child), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "Other provider commit"], check=True)
        self.output = self.base / "wrong-provider"
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_new_recorder_does_not_reinterpret_schema_one_inputs(self):
        snapshot = json.loads(self.inputs.read_text())
        snapshot["schema_version"] = 1
        self.inputs.write_text(json.dumps(snapshot))
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")
        self.assertIn("requires input schema 2", " ".join(self.record()["missing"]))

    def test_malformed_or_noninteger_input_schema_is_incomplete_not_a_process_crash(self):
        snapshot = json.loads(self.inputs.read_text())
        for label, value in (("array", []), ("float", {**snapshot, "schema_version": 2.0})):
            with self.subTest(value=label):
                self.inputs.write_text(json.dumps(value))
                self.output = self.base / label
                self.assertEqual(self.invoke().returncode, 1)
                self.assertEqual(self.record()["status"], "incomplete")
                self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_build_binds_input_tools_configuration_logs_and_discovered_artifacts(self):
        result = self.invoke("build", "--profile", "debug")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = self.record()
        self.assertEqual(record["status"], "observed")
        self.assertEqual(record["source"]["status"], "verified-before-and-after")
        self.assertEqual(record["input"]["vanllia_commit"], self.commit)
        self.assertEqual(record["input"]["inventory_sha256"], hashlib.sha256(self.inputs.read_bytes()).hexdigest())
        self.assertEqual((self.output / record["input"]["path"]).read_bytes(), self.inputs.read_bytes())
        self.assertTrue(record["input"]["verified"])
        self.assertEqual(record["configuration"]["profile"], "debug")
        self.assertEqual(record["configuration"]["environment"]["RUSTUP_TOOLCHAIN"], "fixture-toolchain")
        self.assertEqual(record["execution"]["argv"], ["nu", "scripts/build.nu", "debug"])
        self.assertEqual(record["execution"]["returncode"], 0)
        self.assertEqual(record["execution"]["status"], "exited")
        self.assertFalse(record["release_approved"])
        self.assertFalse(record["is_release_manifest"])
        for stream in ("stdout", "stderr"):
            item = record["execution"][stream]
            self.assertEqual(item["sha256"], hashlib.sha256((self.output / item["path"]).read_bytes()).hexdigest())
        for artifact in record["artifacts"]:
            self.assertEqual(artifact["status"], "observed")
            self.assertEqual(artifact["sha256"], hashlib.sha256(Path(artifact["reported_path"]).read_bytes()).hexdigest())
        self.assertIn("artifact freshness", record["not_evaluated"])

    def test_python_records_a_real_synthetic_unittest_command(self):
        result = self.invoke("python-tests")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = self.record()
        self.assertEqual(record["status"], "observed")
        self.assertIn("Ran 1 test", (self.output / "execution.stderr.log").read_text())
        self.assertEqual(record["artifacts"], [])
        self.assertIn("test case counts and coverage", record["not_evaluated"])

    def test_failed_test_keeps_logs_and_process_failure(self):
        path = self.root / "tests/test_sample.py"
        path.write_text(path.read_text().replace("assertTrue(True)", "assertTrue(False)"))
        self.freeze()
        self.assertEqual(self.invoke("python-tests").returncode, 1)
        self.assertEqual(self.record()["status"], "failed")
        self.assertEqual(self.record()["execution"]["returncode"], 1)
        self.assertIn("FAILED", (self.output / "execution.stderr.log").read_text())

    def test_rust_action_reuses_the_existing_locked_host_command(self):
        self.assertEqual(self.invoke("rust-tests").returncode, 0)
        command = self.record()["execution"]["argv"]
        self.assertEqual(command, ["cargo", "test", "--locked", "--offline", "-p", "vanllia-composition", "-p", "vanllia-image", "--tests", "--target", "x86_64-unknown-linux-gnu"])

    def test_changed_snapshot_cannot_assert_approval_or_verification(self):
        snapshot = json.loads(self.inputs.read_text())
        snapshot["release_approved"] = True
        self.inputs.write_text(json.dumps(snapshot))
        self.assertEqual(self.invoke().returncode, 1)
        record = self.record()
        self.assertFalse(record["input"]["verified"])
        self.assertFalse(record["release_approved"])
        self.assertEqual(record["execution"]["status"], "not-run")
        self.assertIn("regenerated", " ".join(record["missing"]))

    def test_dirty_source_prevents_execution(self):
        (self.root / "tracked.txt").write_text("local edit")
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_changed_source_commit_prevents_execution(self):
        (self.root / "tracked.txt").write_text("next source")
        self.git("add", "tracked.txt")
        self.git("commit", "--quiet", "-m", "Next synthetic input")
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_recorder_bytes_must_belong_to_the_input_commit(self):
        path = self.root / "scripts/record-evidence.py"
        path.write_bytes(path.read_bytes() + b"\n")
        self.freeze()
        self.assertEqual(self.invoke().returncode, 1)
        self.assertIn("recorder bytes", " ".join(self.record()["missing"]))

    def test_successful_process_with_missing_or_directory_artifact_is_incomplete(self):
        for mode in ("missing-artifact", "directory-artifact"):
            with self.subTest(mode=mode):
                self.output = self.base / mode
                self.env["EVIDENCE_FIXTURE_MODE"] = mode
                self.assertEqual(self.invoke().returncode, 1)
                record = self.record()
                self.assertEqual(record["execution"]["returncode"], 0)
                self.assertEqual(record["execution"]["status"], "exited")
                self.assertEqual(record["status"], "incomplete")
                self.assertEqual(record["artifacts"][1]["status"], "missing")

    def test_build_process_failure_is_retained(self):
        self.env["EVIDENCE_FIXTURE_MODE"] = "build-failure"
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["returncode"], 7)
        self.assertEqual(self.record()["status"], "failed")

    def test_invalid_build_plan_or_result_is_not_observed_success(self):
        for mode in ("invalid-plan", "invalid-result"):
            with self.subTest(mode=mode):
                self.env["EVIDENCE_FIXTURE_MODE"] = mode
                self.output = self.base / mode
                self.assertEqual(self.invoke().returncode, 1)
                self.assertEqual(self.record()["status"], "incomplete")

    def test_tool_probe_failure_prevents_execution(self):
        self.env["EVIDENCE_FIXTURE_MODE"] = "probe-failure"
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_observed_toolchain_must_match_the_configured_selection(self):
        self.env["EVIDENCE_FIXTURE_MODE"] = "wrong-toolchain"
        self.assertEqual(self.invoke("rust-tests").returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")
        self.assertIn("active Rust toolchain", " ".join(self.record()["missing"]))

    def test_missing_configured_toolchain_is_explicit_and_prevents_execution(self):
        (self.root / "rust-toolchain.toml").unlink()
        self.freeze()
        self.assertEqual(self.invoke("rust-tests").returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")
        self.assertIn("missing the configured root Rust toolchain", " ".join(self.record()["missing"]))

    def test_build_plan_accepts_the_same_source_through_a_symlink(self):
        (self.base / "source-alias").symlink_to(self.root, target_is_directory=True)
        self.env["EVIDENCE_FIXTURE_MODE"] = "symlink-plan"
        self.assertEqual(self.invoke().returncode, 0)
        plan = self.record()["configuration"]["build_plan"]
        self.assertEqual(Path(plan["cwd"]).resolve(), self.root)

    def test_artifact_hash_is_bound_to_the_recorded_resolved_path(self):
        self.env["EVIDENCE_FIXTURE_MODE"] = "symlink-artifact"
        self.assertEqual(self.invoke().returncode, 0)
        artifact = self.record()["artifacts"][1]
        self.assertNotEqual(artifact["reported_path"], artifact["resolved_path"])
        self.assertEqual(artifact["sha256"], hashlib.sha256(Path(artifact["resolved_path"]).read_bytes()).hexdigest())

    def test_nested_gitlink_checkout_is_checked_and_a_dirty_child_is_refused(self):
        child = self.root / "components/child"
        child.mkdir(parents=True)
        subprocess.run(["git", "init", "--quiet", str(child)], check=True)
        (child / "Cargo.toml").write_text("[workspace]\n")
        (child / "tracked.txt").write_text("child source")
        subprocess.run(["git", "-C", str(child), "add", "."], check=True)
        subprocess.run(["git", "-C", str(child), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "Child fixture"], check=True)
        child_commit = subprocess.check_output(["git", "-C", str(child), "rev-parse", "HEAD"], text=True).strip()
        (self.root / ".gitmodules").write_text('[submodule "child"]\npath = components/child\nurl = https://example.invalid/child.git\n')
        self.git("update-index", "--add", "--cacheinfo", f"160000,{child_commit},components/child")
        self.freeze()
        self.assertEqual(self.invoke().returncode, 0)
        self.assertEqual([item["path"] for item in self.record()["source"]["after"]], [".", "components/child"])
        (child / "tracked.txt").write_text("uncommitted child")
        self.output = self.base / "dirty-child"
        self.assertEqual(self.invoke().returncode, 1)
        self.assertEqual(self.record()["execution"]["status"], "not-run")

    def test_source_change_during_successful_command_breaks_binding(self):
        self.env["EVIDENCE_FIXTURE_MODE"] = "mutate-source"
        self.assertEqual(self.invoke().returncode, 1)
        record = self.record()
        self.assertEqual(record["execution"]["returncode"], 0)
        self.assertEqual(record["status"], "incomplete")
        self.assertEqual(record["source"]["status"], "unverified")
        self.assertIn("post-run source check", " ".join(record["missing"]))

    def test_existing_output_and_output_inside_source_are_refused(self):
        self.output.mkdir()
        sentinel = self.output / "record.json"
        sentinel.write_text("keep")
        self.assertEqual(self.invoke().returncode, 2)
        self.assertEqual(sentinel.read_text(), "keep")
        self.output = self.root / "new-evidence"
        self.assertEqual(self.invoke().returncode, 2)
        self.assertFalse(self.output.exists())

    def test_no_manual_success_or_arbitrary_command_interface(self):
        for extra in (("--passed",), ("--command", "true"), ("--profile", "fast")):
            with self.subTest(extra=extra):
                self.assertEqual(self.invoke("build", *extra).returncode, 2)
                self.assertFalse(self.output.exists())
        self.assertEqual(self.invoke("python-tests", "--profile", "debug").returncode, 2)
