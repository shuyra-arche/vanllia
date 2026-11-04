"""Injection / register constants shared by drivers and oracles."""

CQB, CQH, CQT, CQCSR = 0x18, 0x20, 0x24, 0x48
ENABLE, ON, BUSY, MEM_FAULT = 1, 1 << 16, 1 << 17, 1 << 8

FAILURE = "virtio-blk: binding 1 stage Stop failed party=4 op=2 code=4"
