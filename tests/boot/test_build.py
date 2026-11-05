import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from tests.support.boot import BootCase, NU

class BuildEntry(BootCase):
    def test_plan(self):
        r = self.invoke("build.nu", "release", "--plan")
        self.assertEqual(r.returncode, 0, r.stderr)
        d = json.loads(r.stdout)
        self.assertEqual(d["cwd"], str(self.root))
        self.assertEqual(d["product"], ["cargo", "image", "build", "product", "release"])
        self.assertIn("--release", d["kernel"])
        self.assertEqual(self.calls(), [])


    def test_plan_debug(self):
        d = json.loads(self.invoke("build.nu", "debug", "--plan").stdout)
        self.assertNotIn("--release", d["kernel"])
        self.assertEqual(self.calls(), [])


    def test_invalid_profile(self):
        r = self.invoke("build.nu", "fast", "--plan")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.calls(), [])


    def test_success(self):
        initrd, elf = self.sample("i.img"), self.sample("k.elf")
        self.image(initrd); self.kelf(elf)
        r = self.invoke("build.nu", "release")
        self.assertEqual(r.returncode, 0, r.stderr)
        rec = json.loads(r.stdout)
        self.assertEqual(rec, {"scene": "product", "profile": "release",
                               "initrd": str(initrd), "kernel": str(elf)})
        calls = self.calls()
        self.assertEqual(len(calls), 2)
        self.assertTrue(all(c["cwd"] == str(self.root) for c in calls))
        self.assertEqual(calls[1]["argv"][:8], ["build", "--locked", "--manifest-path",
            str(self.root / "components/kernel/Cargo.toml"), "--bin", "shuyra",
            "--target", "riscv64gc-unknown-none-elf"])
        self.assertIn("semihosting", calls[1]["argv"])
        self.assertIn("--release", calls[1]["argv"])


    def test_last_ok(self):
        gone = self.tmp / "gone.img"
        initrd, elf = self.sample("last.img"), self.sample("k.elf")
        self.env["STUB_IMAGE_OUTPUT"] = f"ok  {gone}\nok  {initrd}\nokay ignore this line"
        self.kelf(elf)
        r = self.invoke("build.nu", "debug")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(r.stdout)["initrd"], str(initrd))


    def test_artifact_filter(self):
        elf = self.sample("real.elf")
        bogus = self.sample("bogus.elf")
        self.image(self.sample())
        self.env["STUB_KERNEL_OUTPUT"] = "\n".join([
            json.dumps({"reason": "compiler-artifact", "executable": str(bogus),
                        "target": {"name": "shuyra", "kind": ["test"]}}),
            json.dumps({"reason": "compiler-artifact", "executable": str(bogus),
                        "target": {"name": "other", "kind": ["bin"]}}),
            json.dumps({"reason": "build-finished", "executable": str(bogus)}),
            json.dumps({"reason": "compiler-artifact", "executable": str(elf),
                        "target": {"name": "shuyra", "kind": ["bin"]}})])
        r = self.invoke("build.nu", "release")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(json.loads(r.stdout)["kernel"], str(elf))


    def test_image_failure(self):
        self.image(self.sample(), exit_code=7)
        r = self.invoke("build.nu", "release")
        self.assertEqual(r.returncode, 7)
        self.assertEqual(len(self.calls()), 1)
        self.assertFalse(r.stdout.strip().startswith("{"))


    def test_kernel_failure(self):
        self.image(self.sample())
        self.kelf(self.sample(), exit_code=9)
        r = self.invoke("build.nu", "release")
        self.assertEqual(r.returncode, 9)
        self.assertFalse(r.stdout.strip().startswith("{"))


    def test_image_bad_outputs(self):
        for bad in (self.tmp / "missing", self.tmp / "dir"):
            if bad.name == "dir":
                bad.mkdir()
            self.image(bad)
            r = self.invoke("build.nu", "release")
            self.assertNotEqual(r.returncode, 0, bad)
            self.assertEqual(len(self.calls()), 1)
            self.log.unlink()


    def test_kernel_bad_outputs(self):
        self.image(self.sample())
        for bad in (self.tmp / "missing", self.tmp / "dir"):
            if bad.name == "dir":
                bad.mkdir()
            self.kelf(bad)
            r = self.invoke("build.nu", "release")
            self.assertNotEqual(r.returncode, 0, bad)
            self.log.unlink()
        self.kelf(None)
        self.env["STUB_KERNEL_OUTPUT"] = json.dumps(
            {"reason": "compiler-artifact", "executable": str(self.sample()),
             "target": {"name": "other", "kind": ["bin"]}})
        self.assertNotEqual(self.invoke("build.nu", "release").returncode, 0)


    def test_missing_image_result_is_clear(self):
        self.env["STUB_IMAGE_OUTPUT"] = "okay no artifact here"
        result = self.invoke("build.nu")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cargo image build", result.stderr)
        self.assertEqual(len(self.calls()), 1)
