import re

A_READY = (
    "memory-transfer-fresh: phase=A source=0x61000 target=0x62000 view=120 reader=11"
)


A_TRANSFER = "memory-transfer-fresh: phase=A transfer=MemoryFail::Busy kernel=armed"


A_READER = "memory-transfer-fresh: phase=A reader=11 value=0x0"


A_SOURCE = "memory-transfer-fresh: phase=A source=0x61000 sum=0x7f800"


B_READY = "memory-transfer-fresh: phase=B source=0x63000 target=0x64000 view=122"


B_TRANSFER = "memory-transfer-fresh: phase=B transfer=MemoryFail::Busy kernel=armed"


B_LATE = "memory-transfer-fresh: phase=B late=12 value=0x0"


CLOSED = "memory-transfer-fresh: close done"


READER, LATE = 11, 12


A_TARGET, B_TARGET = 0x62000, 0x64000


A_SOURCE_PAGE = 0x61000


def room(kind, hart, **payload):
    return {"h": hart, "kind": {"room": {kind: payload}}}


def over(line, **fields):
    """把这一行里的具名读数换掉（只用于负对照）。"""
    for key, value in fields.items():
        line = re.sub(rf"(?<![a-z_]){key}=[^ ]+", f"{key}={value}", line, count=1)
    return line


def fault(va, kind="Load", resolved=True, hart=2):
    return {
        "h": hart,
        "kind": {"memory": {"page_fault": {"va": va, "fault": kind, "resolved": resolved}}},
    }


def proven():
    """一跑真导出：等待者在那条关系上入队一次、放行一次，两阶段的页都真的物化过。"""
    return [
        room("spawn", 0, tid=READER),
        fault(A_SOURCE_PAGE, "Store"),
        fault(A_TARGET, resolved=False),
        room("wait", READER, tid=READER, key=5),
        room("wake", 0, tid=READER),
        fault(A_TARGET, resolved=True),
        room("exit", 2, tid=READER, reason=0),
        room("reap", 2, tid=READER),
        fault(A_SOURCE_PAGE, "Load"),
        room("spawn", 0, tid=LATE),
        fault(B_TARGET, resolved=True),
        room("exit", 2, tid=LATE, reason=0),
        room("reap", 2, tid=LATE),
    ]
