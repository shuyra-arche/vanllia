"""Executable stubs and temporary-workspace setup shared by the boot tests.

The actual cases belong to tests.boot; this support object owns no test methods.
Temporary repositories contain scripts and the runtime acceptance package only.
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


from tests.support.paths import ROOT

@unittest.skipIf(NU is None, "nu not on PATH")
class BootCase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.root = self.tmp / "repo"
        shutil.copytree(ROOT / "scripts", self.root / "scripts")
        shutil.copytree(ROOT / "acceptance", self.root / "acceptance")
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


    def firmware(self):
        sbi = self.sample("sbi.bin")
        return sbi, {**self.env, "SHUYRA_SBI": str(sbi)}


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
