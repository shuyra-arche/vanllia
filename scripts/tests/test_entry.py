"""Host-only tests for scripts/build.nu and scripts/qemu-args.nu.

Each test copies `scripts/` into a temporary skeleton repository and runs the
real Nushell binary against a stub `cargo` that only logs and echoes env text.
No QEMU, no real cargo, no network.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

NU = shutil.which("nu")

STUB = textwrap.dedent(
    """\
    #!{python}
    import json, os, sys
    Path = __import__("pathlib").Path
    Path(os.environ["STUB_LOG"]).open("a").write(
        json.dumps({{"argv": sys.argv[1:], "cwd": os.getcwd()}}) + "\\n"
    )
    a = sys.argv[1:]
    if a[:3] == ["image", "build", "product"]:
        p = os.environ.get("STUB_IMAGE_OUTPUT", "")
        if p:
            sys.stdout.write("noise\\n" + p + "\\n")
        sys.exit(int(os.environ.get("STUB_IMAGE_EXIT", "0")))
    if a[:1] == ["build"]:
        p = os.environ.get("STUB_KERNEL_OUTPUT", "")
        if p:
            sys.stdout.write(p + "\\n")
        sys.exit(int(os.environ.get("STUB_KERNEL_EXIT", "0")))
    if a == ["qtest", "--help"]:
        print("--qemu-arg")
        sys.exit(0)
    if a[:1] == ["qtest"]:
        if "QEMU_TEE_LOG" in os.environ:
            Path(os.environ["QEMU_TEE_LOG"]).write_text(os.environ["STUB_CONSOLE"])
            Path(os.environ["QEMU_TEE_STATUS"]).write_text("0")
        print("synthetic cargo-qtest result (no VM)")
        sys.exit(int(os.environ.get("STUB_QTEST_EXIT", "0")))
    sys.exit(3)
    """
).format(python=sys.executable)


@unittest.skipIf(NU is None, "nu not on PATH")
class Entry(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.root = self.tmp / "repo"
        shutil.copytree(Path(__file__).resolve().parents[1], self.root / "scripts")
        (self.root / "components/kernel").mkdir(parents=True)
        (self.root / "components/kernel/Cargo.toml").write_text("[workspace]\n")
        (self.root / ".cargo").mkdir()
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text(STUB)
        cargo.chmod(0o755)
        self.log = self.tmp / "cargo.log"
        self.env = {k: v for k, v in os.environ.items() if not k.startswith("QEMU")}
        self.env = {k: v for k, v in self.env.items() if not k.startswith("SHUYRA")}
        self.env["PATH"] = f"{self.bin}{os.pathsep}{os.environ.get('PATH', '')}"
        self.env["STUB_LOG"] = str(self.log)

    def invoke(self, *args, **kw):
        return subprocess.run(
            [NU, str(self.root / "scripts" / args[0]), *args[1:]],
            cwd=kw.pop("cwd", self.tmp), capture_output=True, text=True,
            env={**self.env, **kw.pop("env", {})}, timeout=60, **kw,
        )

    def calls(self):
        if not self.log.exists():
            return []
        return [json.loads(l) for l in self.log.read_text().splitlines()]

    def sample(self, name="file"):
        p = self.tmp / "abs with spaces" / name
        p.parent.mkdir(exist_ok=True)
        p.write_text("x")
        return p

    def image(self, p, exit_code=0):
        self.env["STUB_IMAGE_OUTPUT"] = f"ok  {p}"
        self.env["STUB_IMAGE_EXIT"] = str(exit_code)

    def kelf(self, p, exit_code=0):
        self.env["STUB_KERNEL_OUTPUT"] = json.dumps(
            {"reason": "compiler-artifact", "executable": str(p),
             "target": {"name": "shuyra", "kind": ["bin"]}})
        self.env["STUB_KERNEL_EXIT"] = str(exit_code)

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

    def firmware(self):
        sbi = self.sample("sbi.bin")
        return sbi, {**self.env, "SHUYRA_SBI": str(sbi)}

    def test_firmware_bad(self):
        missing = self.invoke("qemu-args.nu", "--board-only")
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("SHUYRA_SBI", missing.stderr)
        for bad in (self.tmp / "missing", self.tmp / "dir"):
            if bad.name == "dir":
                bad.mkdir()
            r = subprocess.run([NU, str(self.root / "scripts/qemu-args.nu"), "--board-only"],
                               capture_output=True, text=True,
                               env={**self.env, "SHUYRA_SBI": str(bad)})
            self.assertNotEqual(r.returncode, 0, bad)

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

    def test_qtest_early_failures(self):
        for args in (["--scene", "accept"], ["--profile", "fast"]):
            r = self.invoke("qtest.nu", *args)
            self.assertNotEqual(r.returncode, 0, args)
            self.assertEqual(self.calls(), [])
        r = self.invoke("qtest.nu", "--scene", "product")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.calls(), [])
        r = self.invoke("qtest.nu", "--help")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("qtest.nu", r.stdout)
        self.assertEqual(self.calls(), [])

    def test_missing_image_result_is_clear(self):
        self.env["STUB_IMAGE_OUTPUT"] = "okay no artifact here"
        result = self.invoke("build.nu")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cargo image build", result.stderr)
        self.assertEqual(len(self.calls()), 1)

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

    def prepare_qtest(self):
        _, env = self.firmware()
        self.env.update(env)
        self.env["TRACE_OUT"] = str(self.tmp / "trace")
        self.image(self.sample("non-default initrd.img"))
        self.env["STUB_CONSOLE"] = (
            "boot: entry task=7\n"
            "exit tid=7 reason=0x0 note: system: done\n"
            "memory: frame_allocations before=42 after=42\n")
        git = self.bin / "git"
        git.write_text("#!/bin/sh\nprintf '3aa0a155f825fbf377fc8d04009472ca67cbbf8c\\n'\n")
        git.chmod(0o755)

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


if __name__ == "__main__":
    unittest.main()
