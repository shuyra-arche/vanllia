#!/usr/bin/env python3
"""Call Source's existing product driver and judge with their original semantics."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    source, initrd, firmware = map(Path, sys.argv[1:4])
    harts = sys.argv[4]
    evidence = Path.cwd()
    if digest(source / "SBI.bin") != digest(firmware):
        raise RuntimeError("Source runner firmware differs from verified input")
    before = digest(initrd)
    env = dict(os.environ, TRACE_OUT=str(evidence), QEMU_SMP=harts,
               QEMU_TEE_LOG=str(evidence / "console.log"), CARGO_NET_OFFLINE="true",
               VANLLIA_WORK=str(source / "target/vanllia-product"), VANLLIA_OUTPUT=str(initrd))
    command = ["nu", str(source / "scripts/qtest.nu"), "--package", "kernel", "--scene", "product",
               "--feed-script", str(source / "programs/tests/terminal/session.py"),
               "--", "--test", "embedded", "--features", "semihosting", "--verbose"]
    (evidence / "command.json").write_text(json.dumps(command, indent=2))
    # The existing QEMU shim explicitly changes cwd to QTEST_SOURCE_ROOT.
    export = source / "sqware-diagnose.jsonl"
    if export.exists():
        raise RuntimeError("stale structured export; refusing reuse")
    result = subprocess.run(command, cwd=source, env=env)
    if export.exists():
        export.rename(evidence / export.name)
    if result.returncode:
        raise RuntimeError(f"original Source qtest driver failed: {result.returncode}")
    if digest(initrd) != before:
        raise RuntimeError("Source driver rebuilt different image bytes")
    log = (evidence / "stdout.log").read_text(errors="replace")
    reported = re.findall(r"qtest: embedded: [0-9]+ test\(s\) in `([^`]+)`", log)
    if len(reported) != 1:
        raise RuntimeError("missing exact original runner test executable report")
    path = Path(reported[0])
    (evidence / "test-kernel.json").write_text(json.dumps({
        "cargo_reported_executable": str(path), "sha256": digest(path),
        "harts": int(harts), "original_driver_exit": result.returncode,
        "initrd_sha256": before,
    }, indent=2))


if __name__ == "__main__":
    main()
