import re

READER = "memory-transfer-tlb: reader tid=2 va=0x58000 view=39 budget=2000000"


FACTS = (
    "memory-transfer-tlb: reader tid=2 warm=0x7 warm_at=0x1201a sample=0x1206a "
    "sepc=0x1206a cause=13 stval=0x58000 byte=0x0 status=1 deadline=0x2a2a42 "
    "sstatus=0x200000022 stvec=0xfffffffffffff000"
)


CAUGHT = "memory-transfer-tlb: fault caught at the sampled load"


CLOSED = "memory-transfer-tlb: close done"


LAUNCHER, RUNNER = 3, 1


def room(kind, hart, **payload):
    return {"h": hart, "kind": {"room": {kind: payload}}}


def fault(va, kind="Store", hart=LAUNCHER):
    return {
        "h": hart,
        "kind": {"memory": {"page_fault": {"va": va, "fault": kind, "resolved": True}}},
    }


def proven():
    """一跑真导出：拉起那一笔在控制者那颗 hart，收工两笔在读者那颗。"""
    return [
        room("spawn", LAUNCHER, tid=2),
        fault(0x58000),
        room("exit", RUNNER, tid=2, reason=0),
        room("reap", RUNNER, tid=2),
    ]
