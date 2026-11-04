"""Runtime run adapter for the memory-transfer fresh scene."""

import json, os, pathlib

from acceptance.evidence.kernel_events import Traces
from acceptance.oracles.memory_transfer.fresh import judge
from acceptance.transport.console import Console


def run(conn, capture):
    pending = pathlib.Path(os.environ["SCENE_SUMMARY_PENDING"])
    diagnose = os.environ.get("VANLLIA_DIAGNOSE", "")
    assert diagnose and os.path.isabs(diagnose), "VANLLIA_DIAGNOSE must be an absolute path"
    trace_dir = pathlib.Path(capture).parent
    console = Console(conn, capture)
    trace = Traces(pathlib.Path(diagnose))
    try:
        summary = judge(console, trace)
        pending.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    finally:
        try:
            os.replace(trace.path, trace_dir / "scene-memory-transfer-fresh-diagnose.jsonl")
        except FileNotFoundError:
            pass
