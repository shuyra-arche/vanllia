def valid_evidence():
    console = "\n".join(
        [
            "boot: entry task=10",
            "rtc-root: root task=10",
            "rtc-root: old task=17 team=21",
            "system: static construct name=rtc task=17 team=21 owner=10",
            "system: static construct name=daemon task=18 team=22 owner=10",
            "system: static construct name=daemon-ctl-rtc task=30 team=35 owner=10",
            "system: activate daemon tid=18",
            "daemon-rtc: observed old task=17 before=100",
            "rtc-root: request controller=30 old task=17 before=100",
            "rtc-root: kill old task=17 team=21 before=100",
            "system: retired name=rtc task=17 team=21",
            "system: recover admitted target=rtc caller=18",
            "rtc-root: retired old task=17 team=21",
            "system: static construct name=rtc task=31 team=40 owner=10",
            "rtc-root: new task=31 team=40",
            "daemon-rtc: proof old task=17 before=100 new task=31 after=120",
            "exit tid=30 reason=0x0 note: daemon-ctl-rtc: done",
            "system: retired name=daemon task=18 team=22",
            "system: retired name=rtc task=31 team=40",
            "exit tid=10 reason=0x0 note: daemon-rtc: done",
            "task: all tasks exited, system halted",
            "memory: frame_allocations before=87 after=87",
        ]
    )
    events = [
        {"h": 1, "when": 1000, "kind": {"room": {"spawn": {"tid": 17}}}},
        {"h": 2, "when": 1100, "kind": {"room": {"spawn": {"tid": 18}}}},
        {"h": 3, "when": 1200, "kind": {"room": {"spawn": {"tid": 30}}}},
        {"h": 1, "when": 2000, "kind": {"room": {"reap": {"tid": 17}}}},
        {"h": 1, "when": 3000, "kind": {"room": {"spawn": {"tid": 31}}}},
        {"h": 3, "when": 3200, "kind": {"room": {"reap": {"tid": 30}}}},
        {"h": 2, "when": 3300, "kind": {"room": {"reap": {"tid": 18}}}},
        {"h": 1, "when": 3400, "kind": {"room": {"reap": {"tid": 31}}}},
        {"h": 0, "when": 4000, "kind": {"halt": "halt"}},
    ]
    qtest = "test embedded::tests::scene ... ok\ntest result: ok. 1 passed; 0 failed"
    return console, events, qtest, 0
