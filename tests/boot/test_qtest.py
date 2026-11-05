import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from tests.support.boot import BootCase, NU

class QtestEntry(BootCase):
    def test_qtest_early_failures(self):
        for args in (["--scene", "accept"], ["--profile", "fast"]):
            r = self.invoke("qtest.nu", *args)
            self.assertNotEqual(r.returncode, 0, args)
            self.assertEqual(self.calls(), [])
        r = self.invoke("qtest.nu", "--scene", "product",
                        env={"SHUYRA_SBI": str(self.tmp / "missing")})
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.calls(), [])
        r = self.invoke("qtest.nu", "--help")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("qtest.nu", r.stdout)
        self.assertEqual(self.calls(), [])

    def test_qtest_default_firmware_reaches_board_only_runner_arguments(self):
        self.prepare_qtest()
        self.env.pop("SHUYRA_SBI")
        result = self.invoke("qtest.nu", "--scene", "product", "--feed-after", "0")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        args = self.calls()[-1]["argv"]
        self.assertEqual(args.count("--qemu-arg=-bios"), 1)
        index = args.index("--qemu-arg=-bios")
        self.assertEqual(args[index + 1], "--qemu-arg=default")
        self.env["STUB_QTEST_EXIT"] = "1"
        result = self.invoke("qtest.nu", "--scene", "product", "--feed-after", "0")
        self.assertEqual(result.returncode, 1, result.stderr + result.stdout)
        self.assertTrue((self.tmp / "trace/scene-product-qtest.log").is_file())


    def test_qtest_uses_vanllia_image_and_pinned_kernel_workspace(self):
        self.prepare_qtest()
        result = self.invoke("qtest.nu", "--scene", "product", "--feed-after", "0")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        calls = self.calls()
        self.assertEqual(calls[1]["argv"], ["image", "build", "product", "release"])
        self.assertEqual(calls[1]["cwd"], str(self.root))
        self.assertEqual(calls[2]["cwd"], str(self.root / "components/kernel"))
        args = calls[2]["argv"]
        self.assertEqual(args.count("--qemu-arg=-initrd"), 1)
        self.assertIn("--qemu-arg=" + str(self.tmp / "abs with spaces/non-default initrd.img"), args)
        self.assertIn("-r", args)
        self.assertEqual(args.count("--qemu-arg=-serial"), 1)
        self.assertIn("--package", args)
        self.assertFalse((self.tmp / "trace/scene-product-qtest.log").exists())


    def test_qtest_rejects_frame_leak_and_wrong_entry_even_with_raw_success(self):
        self.prepare_qtest()
        good = self.env["STUB_CONSOLE"]
        for console in (good.replace("after=42", "after=43"),
                        good.replace("exit tid=7", "exit tid=1")):
            self.env["STUB_CONSOLE"] = console
            result = self.invoke("qtest.nu", "--scene", "product", "--feed-after", "0")
            self.assertNotEqual(result.returncode, 0, result.stderr + result.stdout)
            self.assertTrue((self.tmp / "trace/scene-product-qtest.log").is_file())


    def test_qtest_keeps_raw_failure_and_health_sentinel(self):
        self.prepare_qtest()
        self.env["STUB_QTEST_EXIT"] = "101"
        result = self.invoke("qtest.nu", "--scene", "product", "--feed-after", "0")
        self.assertEqual(result.returncode, 101, result.stderr + result.stdout)
        result = self.invoke("qtest.nu")
        self.assertEqual(result.returncode, 101, result.stderr + result.stdout)
        args = self.calls()[-1]["argv"]
        self.assertNotIn("-r", args)
        self.assertNotIn("--qemu-arg=-initrd", args)
        self.assertTrue((self.tmp / "trace/qtest.log").is_file())
