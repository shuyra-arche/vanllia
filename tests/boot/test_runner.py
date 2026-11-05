import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from tests.support.boot import BootCase, NU

class RunnerEntry(BootCase):
    def test_boot_propagates_stub_exit_and_runner_freezes_relative_inputs(self):
        _, env = self.firmware()
        elf = self.sample("kernel.elf")
        image = self.sample("initrd.img")
        qemu = self.bin / "qemu-system-riscv64"
        qemu.write_text(f"#!{sys.executable}\n" +
            "import json, os, pathlib, sys\n" +
            "pathlib.Path(os.environ['STUB_VM_LOG']).write_text(json.dumps(sys.argv[1:]))\n" +
            "print('synthetic boot transport, no VM')\n" +
            "sys.exit(int(os.environ['STUB_VM_EXIT']))\n")
        qemu.chmod(0o755)
        log = self.tmp / "vm.log"
        env.update({"SHUYRA_INITRD": str(image.relative_to(self.tmp)),
                    "SHUYRA_SBI": str(Path(env["SHUYRA_SBI"]).relative_to(self.tmp)),
                    "QEMU_SETTLE": "0", "QEMU_SEED": "123",
                    "TRACE_OUT": str(self.tmp / "trace"),
                    "STUB_VM_LOG": str(log), "STUB_VM_EXIT": "7"})
        result = self.invoke("boot.nu", str(elf), env=env, input="")
        self.assertEqual(result.returncode, 7, result.stderr + result.stdout)
        result = self.invoke("runner.nu", str(elf), env=env, input="")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        args = json.loads(log.read_text())
        self.assertEqual(args[args.index("-bios") + 1], str(self.tmp / env["SHUYRA_SBI"]))
        self.assertEqual(args[args.index("-initrd") + 1], str(image))
        self.assertIn("7", result.stdout)
        self.assertTrue(list((self.tmp / "trace").glob("console-123-*.log")))

    def test_default_firmware_survives_runner_cwd_and_startup_errors_are_visible(self):
        elf = self.sample("kernel.elf")
        image = self.sample("initrd.img")
        log = self.tmp / "vm.log"
        qemu = self.bin / "qemu-system-riscv64"
        qemu.write_text(f"#!{sys.executable}\n" +
            "import json, os, pathlib, sys\n" +
            "pathlib.Path(os.environ['STUB_VM_LOG']).write_text(json.dumps(sys.argv[1:]))\n" +
            "print('synthetic startup error: firmware unavailable (no VM)', file=sys.stderr)\n" +
            "sys.exit(1)\n")
        qemu.chmod(0o755)
        env = {"SHUYRA_INITRD": str(image), "QEMU_SETTLE": "0", "QEMU_SEED": "456",
               "TRACE_OUT": str(self.tmp / "trace"), "STUB_VM_LOG": str(log)}
        for entry in ("boot.nu", "runner.nu"):
            with self.subTest(entry=entry):
                result = self.invoke(entry, str(elf), env=env, input="")
                # runner remains an observer; boot preserves the process failure.
                self.assertEqual(result.returncode, 1 if entry == "boot.nu" else 0,
                                 result.stderr + result.stdout)
                args = json.loads(log.read_text())
                self.assertEqual(args.count("-bios"), 1)
                self.assertEqual(args[args.index("-bios") + 1], "default")
                self.assertEqual(args[args.index("-initrd") + 1], str(image))
                self.assertIn("firmware unavailable", result.stderr + result.stdout)
                if entry == "boot.nu":
                    self.assertFalse((self.tmp / "trace").exists())
        archives = list((self.tmp / "trace").glob("console-456-*.log"))
        self.assertEqual(len(archives), 1)
        self.assertIn("firmware unavailable", archives[0].read_text())

    def test_invalid_custom_firmware_fails_before_any_transport_launch(self):
        elf = self.sample("kernel.elf")
        image = self.sample("initrd.img")
        for entry in ("boot.nu", "runner.nu"):
            result = self.invoke(entry, str(elf), env={
                "SHUYRA_INITRD": str(image), "SHUYRA_SBI": str(self.tmp / "missing"),
                "QEMU_SETTLE": "0", "TRACE_OUT": str(self.tmp / "trace")}, input="")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("SHUYRA_SBI", result.stderr)
        self.assertFalse((self.tmp / "trace").exists())
