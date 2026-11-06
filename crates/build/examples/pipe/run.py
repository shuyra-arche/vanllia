#!/usr/bin/env python3
"""Adapter to the provider's unchanged embedded-test scene and declared probe.

No protocol driver is recreated here. Cargo-qemu-test owns test discovery,
the 60-second guest limit and its native result. The provider's probe/Root own
all pipe assertions; required marker names are read from its own README.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def main():
    kernel, pipe, initrd, firmware = map(Path, sys.argv[1:5])
    harts = int(sys.argv[5])
    evidence = Path.cwd()
    lock = (kernel / "Cargo.lock").read_bytes()
    env = dict(os.environ, QEMU_TEE_LOG=str(evidence / "console.log"), CARGO_NET_OFFLINE="true")
    command = ["cargo", "qtest", "--target", "riscv64gc-unknown-none-elf", "--release",
               "--features", "semihosting", "--test", "embedded", "--timeout", "60",
               "--qemu", str(Path(__file__).with_name("qemu.sh")), "--verbose"]
    board = ["-machine", "virt", "-bios", str(firmware), "-m", "256", "-smp", str(harts),
             "-display", "none", "-serial", "stdio", "-monitor", "none", "-no-reboot",
             "-initrd", str(initrd)]
    command += [f"--qemu-arg={arg}" for arg in board]
    # semihosting exports are relative to the runner's working directory.
    # Kernel sources live in the prepared context; archive the actual export here.
    export = kernel / "sqware-diagnose.jsonl"
    if export.exists():
        raise RuntimeError("stale structured export in prepared source; refusing reuse")
    (evidence / "command.json").write_text(json.dumps(command, indent=2))
    (evidence / "versions.txt").write_bytes(
        subprocess.check_output(["cargo", "qtest", "--version"]) +
        subprocess.check_output(["qemu-system-riscv64", "--version"]))
    with (evidence / "framework.log").open("w") as log:
        status = subprocess.run(command, cwd=kernel, env=env, stdout=log, stderr=subprocess.STDOUT)
    if export.exists():
        export.rename(evidence / export.name)
    if (kernel / "Cargo.lock").read_bytes() != lock:
        raise RuntimeError("runner changed prepared Cargo.lock")
    if status.returncode:
        raise RuntimeError(f"original cargo-qemu-test failed: {status.returncode}; see framework.log")
    console = (evidence / "console.log").read_text(errors="replace")
    framework = (evidence / "framework.log").read_text(errors="replace")
    if not re.search(r"1 passed; 0 failed", framework):
        raise RuntimeError("missing original framework success")
    readme = (pipe / "integration/README.md").read_text()
    markers = readme.split("### Markers", 1)[1].split("```text", 1)[1].split("```", 1)[0].strip().splitlines()
    for marker in markers:
        pattern = re.escape(marker).replace(re.escape("<decimal>"), "[0-9]+")
        if len(re.findall("^" + pattern + "$", console, re.M)) != 1:
            raise RuntimeError(f"missing/repeated provider marker: {marker}")
    semantic = [line for line in console.splitlines() if line.startswith(("PIPE:", "PIPE-INTEGRATION:"))]
    if semantic[-1] != "PIPE-INTEGRATION: PASS" or "FAIL" in "\n".join(semantic):
        raise RuntimeError("semantic completion failed")
    for marker in ["PIPE-ROOT: SHUTDOWN_OK", "task: all tasks exited, system halted"]:
        if console.count(marker) != 1:
            raise RuntimeError(f"missing/repeated shutdown: {marker}")
    frames = re.findall(r"^memory: frame_allocations before=([0-9]+) after=([0-9]+)$", console, re.M)
    if len(frames) != 1 or int(frames[0][0]) == 0 or frames[0][0] != frames[0][1]:
        raise RuntimeError(f"frame baseline not restored: {frames}")
    rows = [json.loads(line) for line in (evidence / "sqware-diagnose.jsonl").read_text().splitlines()]
    events = [row["kind"] for row in rows if "kind" in row]
    rooms = [event["room"] for event in events if "room" in event]
    spawned = {event["spawn"]["tid"] for event in rooms if "spawn" in event}
    exits = {event["exit"]["tid"]: event["exit"]["reason"] for event in rooms if "exit" in event}
    reaped = {event["reap"]["tid"] for event in rooms if "reap" in event}
    if not spawned or spawned != reaped or not set(exits).issubset(spawned):
        raise RuntimeError(f"native task accounting incomplete: spawn={spawned} reap={reaped} exit={exits}")
    if any("fault_killed" in event for event in rooms) or {"halt": "panic"} in events:
        raise RuntimeError("native fault/panic")
    if any(reason != 0 for reason in exits.values()) or {"halt": "halt"} not in events:
        raise RuntimeError("native exit/halt failed")
    for note in ["pipe-root: shutdown completed", "pipe-probe: pass", "pipe-peer: done"]:
        if len(re.findall(r"^exit tid=[0-9]+ reason=0x0 note: " + re.escape(note) + "$", console, re.M)) != 1:
            raise RuntimeError(f"missing named zero exit: {note}")
    resident = re.findall(r"^system: minted pipe tid=([0-9]+)$", console, re.M)
    if len(resident) != 1 or int(resident[0]) not in reaped or int(resident[0]) in exits:
        raise RuntimeError("resident retirement/native accounting mismatch")
    if len(re.findall(r"^system: retired name=pipe task=" + resident[0] + r" team=[0-9]+$", console, re.M)) != 1:
        raise RuntimeError("missing explicit System retirement")
    if len(spawned) != 6 or len(exits) != 5:
        raise RuntimeError("unexpected deployment task inventory")
    summary = {"harts": harts, "framework": "1 passed; 0 failed", "semantic": "PASS",
               "frames": frames, "spawned": sorted(spawned), "reaped": sorted(reaped),
               "native_exits": exits, "resident_numeric_exit": "unobserved; System-controlled retirement",
               "initrd_sha256": hashlib.sha256(initrd.read_bytes()).hexdigest()}
    (evidence / "verdict.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
