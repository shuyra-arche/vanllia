"""Pure QEMU device-tree parsers; stdlib only."""

import re


def regions(dts):
    leaves = re.findall(r"[^{}]+\{([^{}]*)\}", dts)

    def window(body):
        cells = re.search(r"\breg = <([^>]+)>;", body)[1].split()
        assert len(cells) == 4, "QEMU virt fixture requires 64-bit address/size cells"
        words = [int(cell, 0) for cell in cells]
        return (words[0] << 32) | words[1], (words[2] << 32) | words[3]

    iommus = [window(body) for body in leaves if '"riscv,iommu"' in body]
    memories = [window(body) for body in leaves if 'device_type = "memory"' in body]
    assert len(iommus) == len(memories) == 1, "ambiguous QEMU device tree"
    memory_base, memory_size = memories[0]
    assert memory_size > 0 and (memory_base + memory_size) % 4096 == 0
    return iommus[0][0], memory_base + memory_size


def ecam_region(dts):
    nodes = re.findall(r"[^{}]+\{([^{}]*)\}", dts)
    windows = []
    for body in nodes:
        if '"pci-host-ecam-generic"' not in body:
            continue
        cells = re.search(r"\breg = <([^>]+)>;", body)[1].split()
        assert len(cells) == 4, "expected 64-bit ECAM geometry"
        words = [int(cell, 0) for cell in cells]
        windows.append(((words[0] << 32) | words[1], (words[2] << 32) | words[3]))
    assert len(windows) == 1, "ambiguous physical ECAM"
    return windows[0]
