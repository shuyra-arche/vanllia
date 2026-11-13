#!/usr/bin/env python3
"""Build and execute the native memory composition; preserve exact image evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parent
TARGET = "riscv64gc-unknown-none-elf"

def capsule(elf, version):
    assert elf[:6] == b"\x7fELF\x02\x01"
    entry, table = struct.unpack_from("<QQ", elf, 24)
    width, count = struct.unpack_from("<HH", elf, 54)
    output, regions = bytearray(4096), []
    for i in range(count):
        kind, flags, offset, va, _, size, memory, _ = struct.unpack_from("<IIQQQQQQ", elf, table + i * width)
        if kind != 1 or memory == 0:
            continue
        assert va % 4096 == 0 and size <= memory
        pages, data = (memory + 4095) // 4096, (size + 4095) // 4096
        payload = len(output) if data else 0
        if data:
            output.extend(elf[offset:offset + size])
            output.extend(bytes(payload + data * 4096 - len(output)))
        regions.append((va, pages, data, payload, 1 | (2 if flags & 2 else 0) | (4 if flags & 1 else 0)))
    assert 32 + len(regions) * 40 <= 4096
    struct.pack_into("<8sHBBIQQ", output, 0, b"SQBOOT01", 2, 12, version, len(regions), entry, len(output))
    for i, region in enumerate(regions):
        struct.pack_into("<5Q", output, 32 + i * 40, *region)
    return struct.pack("<II", 4096, len(output)) + bytes(4088) + output

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, help="explicit local provider override")
    parser.add_argument("--profile", choices=["debug", "release"], default="debug")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--capsule-version", type=int, choices=[10,11], default=11)
    args = parser.parse_args()
    command = ["cargo", "build", "--manifest-path", str(ROOT / "Cargo.toml"), "--target", TARGET]
    if args.profile == "release":
        command.append("--release")
    if args.kernel:
        for package, folder in [("kernel", "server"), ("kernel-api", "api"), ("kernel-client", "client")]:
            patch = f'patch."https://github.com/shuyra-arche/kernel.git".{package}.path={json.dumps(str(args.kernel.resolve() / folder))}'
            command.extend(["--config", patch])
    else:
        command.append("--locked")
    if not args.no_build:
        subprocess.run(command, cwd=ROOT.parents[1], check=True)
    binaries = ROOT / "target" / TARGET / args.profile
    evidence = ROOT / "target" / "evidence" / args.profile
    if args.capsule_version != 11:
        evidence = evidence / "reject-v10"
    evidence.mkdir(parents=True, exist_ok=True)
    guest, platform = binaries / "memory-guest", binaries / "memory-platform"
    shutil.copy2(guest, evidence / "guest.elf")
    shutil.copy2(platform, evidence / "platform.elf")
    image = evidence / "initrd.bin"
    image.write_bytes(capsule(guest.read_bytes(), args.capsule_version))
    firmware = Path(os.environ["QEMU_OPENSBI"]).resolve()
    command = [os.environ.get("QEMU_RISCV64", "qemu-system-riscv64"), "-accel", "tcg,thread=single", "-machine", "virt,iommu-sys=on", "-cpu", "rv64", "-smp", "1", "-m", "128M", "-nographic", "-bios", str(firmware), "-kernel", str(platform), "-initrd", str(image)]
    disk = evidence / "disk.raw"
    disk.write_bytes(bytes(1024*1024))
    command.extend(["-drive", f"file={disk},format=raw,if=none,id=disk,aio=threads", "-device", "virtio-blk-pci,drive=disk,disable-legacy=on,iommu_platform=on,ats=off,addr=1,ioeventfd=off,vectors=0"])
    qemu_version = subprocess.check_output([command[0], "--version"], text=True).splitlines()[0]
    assert "version 10.0." in qemu_version, "Only QEMU 10.0.x reset/drain has been reviewed"
    metadata = {"rustc": subprocess.check_output(["rustc", "-Vv"], text=True), "qemu": qemu_version, "command": command, "provider_override": str(args.kernel.resolve()) if args.kernel else None, "sha256": {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [guest, platform, image, firmware, ROOT / "Cargo.lock"]}}
    (evidence / "inputs.json").write_text(json.dumps(metadata, indent=2))
    started = time.monotonic()
    with (evidence / "serial.log").open("wb") as serial, (evidence / "stderr.log").open("wb") as errors:
        try:
            result = subprocess.run(command, stdout=serial, stderr=errors, timeout=120)
            status = result.returncode
        except subprocess.TimeoutExpired:
            status = "timeout"
    log = (evidence / "serial.log").read_text(errors="replace")
    (evidence / "elapsed.json").write_text(json.dumps({"seconds": time.monotonic()-started, "status": status, "disk_sha256": hashlib.sha256(disk.read_bytes()).hexdigest()}))
    if args.capsule_version == 10:
        assert status != "timeout" and "shuyra: boot failed" in log and "assemble boot capsule" in log and "virtual address not in any declared map" in log and "START native-memory" not in log, log
    else:
        required = ["PASS native-memory", "PASS native-version", "PASS native-fault", "PASS native-ipc", "PASS native-swap", "PASS native-virtqueue", "iommu: command engine closed", "task: all tasks exited"]
        assert status == 0 and "FAIL" not in log and all(marker in log for marker in required), log
    print(log)
    print(f"Evidence: {evidence}")

if __name__ == "__main__":
    main()
