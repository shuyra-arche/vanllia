import re

READY = "memory-transfer-race: phase=fault peer=11 va0=0x1d000 va1=0x1e000 read0=0x7f800"


MOVED = "memory-transfer-race: phase=fault transfer=Ok(()) kernel=armed foreign=1"


READ = "memory-transfer-race: phase=fault read1=0x7f800 read0=0x7f800"


TORN = (
    "memory-transfer-race: phase=teardown transfer=Ok(()) kernel=armed "
    "press=2->1 peer1=1->0 released=0x0 resumed=1"
)


TORN_READ = "memory-transfer-race: phase=teardown read0=0x7f800"


JOINED = "memory-transfer-race: phase=fault joined cause=Reap reason=0x0"


CLOSED = "memory-transfer-race: close done"


PEER, VA0, VA1 = 11, 0x1D000, 0x1E000


KEY = 12525070699606571938


def room(kind, hart, **payload):
    return {"h": hart, "kind": {"room": {kind: payload}}}


def fault(va, resolved, hart=1):
    """一次取数缺页；`h` 只是边界读数（判定不认它）。"""
    return {
        "h": hart,
        "kind": {"memory": {"page_fault": {"va": va, "fault": "Load", "resolved": resolved}}},
    }


def over(line, **fields):
    """把这一行里的具名读数换掉（只用于负对照）。"""
    for key, value in fields.items():
        line = re.sub(rf"(?<![a-z_]){key}=[^ ]+", f"{key}={value}", line, count=1)
    return line


def proven():
    """一跑真导出：对端在目标页上先无承载地等一次，被那一次移交放行后物化出同一个承载。"""
    return [
        room("spawn", 1, tid=PEER),
        fault(VA1, False),
        room("wait", 2, tid=PEER, key=KEY),
        room("wake", 3, tid=PEER),
        fault(VA1, True, hart=0),
        room("exit", 0, tid=PEER, reason=0),
        room("reap", 0, tid=PEER),
    ]
