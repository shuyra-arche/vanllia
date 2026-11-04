"""Final memory-transfer phase verdicts over complete observations."""
from acceptance.evidence.kernel_events import room_payload


def require_reaps(trace, want):
    reaps = [tid for spec in want.values() for tid in spec["waits"]]
    missing = [tid for tid in reaps if not trace.room("reap", tid)]
    assert not missing, f"these fault tasks were never reclaimed: {missing}"


def settle_phase(trace, phase, state, spec):
    """每阶段按最终读数重核：入队次数、放行次数、离场原因、回收与等待位身份。"""
    final = state.setdefault("final", {})
    for tid in state["tids"]:
        entry = final.setdefault(str(tid), {})
        entry["waits"] = len(trace.room("wait", tid))
        entry["wakes"] = len(trace.room("wake", tid))
        entry["reaps"] = len(trace.room("reap", tid))
        entry["exits"] = [hex(e["reason"]) for e in trace.room("exit", tid)]
        entry["fault_killed"] = [hex(e["stval"]) for e in trace.room("fault_killed", tid)]
        assert entry["waits"] == spec["waits"][tid], (
            f"phase {phase}: tid {tid} queued {entry['waits']} times, wanted {spec['waits'][tid]}"
        )
        assert entry["wakes"] == spec["wakes"][tid], (
            f"phase {phase}: tid {tid} woke {entry['wakes']} times, wanted {spec['wakes'][tid]}"
        )
        wanted = spec["exits"][tid]
        assert entry["exits"] == ([] if wanted is None else [hex(wanted)]), (
            f"phase {phase}: tid {tid} ended as {entry['exits']}, wanted {wanted}"
        )
        assert entry["reaps"] == 1, (
            f"phase {phase}: tid {tid} reaped {entry['reaps']} times, wanted 1"
        )
    key = state["key"]
    on_key = {
        entry["tid"]
        for event in trace.events
        for entry in [room_payload(event, "wait") or {}]
        if entry.get("key") == key
    }
    assert on_key == set(state["tids"]), (
        f"phase {phase}: the wait slot named {sorted(on_key)}, wanted {state['tids']}"
    )
    state["on_key"] = sorted(on_key)
