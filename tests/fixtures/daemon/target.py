def valid_evidence():
    console = "\n".join(
        [
            "boot: entry task=10",
            "target-root: root task=10",
            "target-root: old task=17 team=21",
            "system: static construct name=fixturetarget task=17 team=21 owner=10",
            "system: static construct name=daemon task=18 team=22 owner=10",
            "system: static construct name=daemon-ctl-target task=30 team=35 owner=10",
            "system: activate daemon tid=18",
            "daemon-target: observed old task=17 helper=25 before=100",
            "target-root: request controller=30 old task=17 helper=25 before=100",
            "target-root: kill old task=17 team=21 before=100",
            "system: retired name=fixturetarget task=17 team=21",
            "system: recover admitted target=fixturetarget caller=18",
            "target-root: retired old task=17 team=21",
            "system: static construct name=fixturetarget task=31 team=40 owner=10",
            "target-root: new task=31 team=40",
            "daemon-target: proof old task=17 helper=25 before=100 new task=31 helper=45 after=120",
            "exit tid=30 reason=0x0 note: daemon-ctl-target: done",
            "system: retired name=daemon task=18 team=22",
            "system: retired name=fixturetarget task=31 team=40",
            "exit tid=10 reason=0x0 note: daemon-target: done",
            "task: all tasks exited, system halted",
            "memory: frame_allocations before=87 after=87",
        ]
    )
    events = [
        {"h": 1, "when": 1000, "kind": {"room": {"spawn": {"tid": 17}}}},
        {"h": 1, "when": 1050, "kind": {"room": {"spawn": {"tid": 25}}}},
        {"h": 2, "when": 1100, "kind": {"room": {"spawn": {"tid": 18}}}},
        {"h": 3, "when": 1200, "kind": {"room": {"spawn": {"tid": 30}}}},
        {"h": 1, "when": 2000, "kind": {"room": {"reap": {"tid": 17}}}},
        {"h": 1, "when": 2100, "kind": {"room": {"reap": {"tid": 25}}}},
        {"h": 1, "when": 3000, "kind": {"room": {"spawn": {"tid": 31}}}},
        {"h": 1, "when": 3050, "kind": {"room": {"spawn": {"tid": 45}}}},
        {"h": 3, "when": 3900, "kind": {"room": {"reap": {"tid": 30}}}},
        {"h": 2, "when": 3800, "kind": {"room": {"reap": {"tid": 18}}}},
        {"h": 1, "when": 4000, "kind": {"room": {"reap": {"tid": 31}}}},
        {"h": 1, "when": 4050, "kind": {"room": {"reap": {"tid": 45}}}},
        {"h": 0, "when": 4100, "kind": {"halt": "halt"}},
    ]
    qtest = "test embedded::tests::scene ... ok\ntest result: ok. 1 passed; 0 failed"
    return console, events, qtest, 0
