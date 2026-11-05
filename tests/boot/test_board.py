import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from tests.support.boot import BootCase, NU

class BoardEntry(BootCase):
    def test_firmware_bad(self):
        missing = self.invoke("qemu-args.nu", "--board-only",
                              env={"SHUYRA_SBI": str(self.tmp / "missing")})
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("SHUYRA_SBI", missing.stderr)
        for bad in (self.tmp / "missing", self.tmp / "dir"):
            if bad.name == "dir":
                bad.mkdir()
            r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only"],
                               capture_output=True, text=True,
                               env={**self.env, "SHUYRA_SBI": str(bad)})
            self.assertNotEqual(r.returncode, 0, bad)

    def test_default_firmware_is_qemu_token_in_board_and_kernel_arguments(self):
        elf = self.sample("kernel.elf")
        for env in ({}, {"SHUYRA_SBI": ""}, {"SHUYRA_SBI": "default"}):
            for argv in (["--board-only"], [str(elf)]):
                with self.subTest(env=env, argv=argv):
                    result = self.invoke("qemu-args.nu", *argv, env=env)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    args = result.stdout.splitlines()
                    self.assertEqual(args.count("-bios"), 1)
                    self.assertEqual(args[args.index("-bios") + 1], "default")
                    self.assertEqual("-kernel" in args, argv != ["--board-only"])

    def test_custom_firmware_path_is_validated_and_default_filename_can_be_explicit(self):
        (self.tmp / "default").write_text("custom firmware")
        for raw in ("./default", "none", "absent"):
            result = self.invoke("qemu-args.nu", "--board-only", env={"SHUYRA_SBI": raw})
            if raw == "./default":
                self.assertEqual(result.returncode, 0, result.stderr)
                args = result.stdout.splitlines()
                self.assertEqual(args[args.index("-bios") + 1], str(self.tmp / "default"))
            else:
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("SHUYRA_SBI", result.stderr)


    def test_board_only(self):
        sbi, env = self.firmware()
        r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only"],
                           capture_output=True, text=True, env=env)
        self.assertEqual(r.returncode, 0, r.stderr)
        args = r.stdout.split("\n")
        self.assertEqual(args.count("-bios"), 1)
        self.assertIn(str(sbi), args)
        self.assertNotIn("-kernel", args)
        self.assertNotIn("-initrd", args)
        self.assertNotIn("-semihosting", args)


    def test_serial_and_iommu(self):
        sbi, env = self.firmware()
        r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only",
                            "--serial", "tcp:127.0.0.1:1234", "--iommu"],
                           capture_output=True, text=True, env=env)
        self.assertEqual(r.returncode, 0, r.stderr)
        args = r.stdout.split("\n")
        self.assertEqual(args.count("-serial"), 1)
        self.assertIn("tcp:127.0.0.1:1234", args)
        self.assertIn("virt,iommu-sys=on", args)
        self.assertEqual(args.count("-m"), 1)


    def test_initrd(self):
        sbi, env = self.firmware()
        img = self.sample("explicit.img")
        r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only"],
                           capture_output=True, text=True,
                           env={**env, "SHUYRA_INITRD": str(img)})
        self.assertNotIn("-initrd", r.stdout.split("\n"))
        elf = self.tmp / "adjacent" / "kernel.elf"
        elf.parent.mkdir()
        elf.write_text("x")
        (elf.parent / "initrd.img").write_text("x")
        r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only",
                            "--serial", "stdio"], capture_output=True, text=True, env=env)
        self.assertNotIn("-initrd", r.stdout.split("\n"))
        r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), str(elf)],
                           capture_output=True, text=True,
                           env={**env, "QEMU_SETTLE": "0"})
        self.assertIn("-initrd", r.stdout.split("\n") if r.stdout else [])


    def test_explicit_initrd_overrides_adjacent_and_must_be_regular(self):
        _, env = self.firmware()
        elf = self.sample("kernel.elf")
        adjacent = self.sample("initrd.img")
        explicit = self.sample("explicit.img")
        for image in (explicit, adjacent.parent, self.tmp / "absent"):
            result = self.invoke("qemu-args.nu", str(elf),
                                 env={**env, "SHUYRA_INITRD": str(image)})
            if image == explicit:
                self.assertEqual(result.returncode, 0, result.stderr)
                args = result.stdout.splitlines()
                self.assertEqual(args[args.index("-initrd") + 1], str(explicit))
                self.assertEqual(args.count("-initrd"), 1)
            else:
                self.assertNotEqual(result.returncode, 0)


    def test_board_defaults_and_overrides_keep_original_order(self):
        _, env = self.firmware()
        result = self.invoke("qemu-args.nu", "--board-only", env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        args = result.stdout.splitlines()
        self.assertEqual(args[args.index("-m") + 1], "256")
        self.assertEqual(args[args.index("-smp") + 1], "4")
        self.assertEqual(args[args.index("-icount") + 1], "auto,sleep=on")
        result = self.invoke("qemu-args.nu", "--board-only", env={**env,
            "QEMU_MEM": "512", "QEMU_SMP": "8", "QEMU_SEED": "123",
            "QEMU_ICOUNT": "", "QEMU_SEMI": "1", "QEMU_GDB": "1"})
        self.assertEqual(result.returncode, 0, result.stderr)
        args = result.stdout.splitlines()
        self.assertEqual(args[args.index("-m") + 1], "512")
        self.assertEqual(args[args.index("-smp") + 1], "8")
        self.assertEqual(args[args.index("-seed") + 1], "123")
        self.assertNotIn("-icount", args)
        self.assertNotIn("-semihosting", args)
        self.assertEqual(args[-2:], ["-s", "-S"])
