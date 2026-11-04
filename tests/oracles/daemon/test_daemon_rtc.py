"""Parser negative controls: each case mutates otherwise valid runner evidence."""

import copy
import sys
import tempfile
import unittest
from pathlib import Path

from acceptance.oracles.daemon import rtc as daemon_rtc
from tests.fixtures.daemon.rtc import valid_evidence




class DaemonRtcVerifier(unittest.TestCase):
    def verify(self, console, events, qtest, raw=0):
        return daemon_rtc.verify(console, events, qtest, raw)

    def test_valid_evidence_passes(self):
        self.verify(*valid_evidence())

    def test_wrong_recover_caller(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("caller=18", "caller=99"),
            events,
            qtest,
            raw,
        )

    def test_missing_daemon_activation(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("system: activate daemon tid=18\n", ""),
            events,
            qtest,
            raw,
        )

    def test_duplicate_daemon_activation(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console + "\nsystem: activate daemon tid=18",
            events,
            qtest,
            raw,
        )

    def test_stale_controller_owner(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("observed old task=17", "observed old task=99"),
            events,
            qtest,
            raw,
        )

    def test_retirement_after_new_construction(self):
        console, events, qtest, raw = valid_evidence()
        retired = "system: retired name=rtc task=17 team=21"
        moved = console.replace(retired + "\n", "")
        marker = "system: static construct name=rtc task=31 team=40 owner=10"
        moved = moved.replace(marker, marker + "\n" + retired)
        self.assertRaises(AssertionError, self.verify, moved, events, qtest, raw)

    def test_admission_before_finalized_retirement(self):
        console, events, qtest, raw = valid_evidence()
        retired = "system: retired name=rtc task=17 team=21"
        admitted = "system: recover admitted target=rtc caller=18"
        moved = console.replace(retired + "\n" + admitted, admitted + "\n" + retired)
        self.assertRaises(AssertionError, self.verify, moved, events, qtest, raw)

    def test_old_reap_after_new_spawn(self):
        console, events, qtest, raw = valid_evidence()
        events = copy.deepcopy(events)
        events[3]["when"] = 3500
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_replacement_keeps_old_team(self):
        console, events, qtest, raw = valid_evidence()
        console = console.replace("new task=31 team=40", "new task=31 team=21")
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_fresh_task_only_is_not_proof(self):
        console, events, qtest, raw = valid_evidence()
        console = "\n".join(
            line for line in console.splitlines() if not line.startswith("daemon-rtc: proof")
        )
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_wrong_construction_owner(self):
        console, events, qtest, raw = valid_evidence()
        console = console.replace(
            "name=rtc task=31 team=40 owner=10", "name=rtc task=31 team=40 owner=99"
        )
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_missing_controller_exit(self):
        console, events, qtest, raw = valid_evidence()
        console = "\n".join(
            line for line in console.splitlines() if "daemon-ctl-rtc: done" not in line
        )
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_frame_regression(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("after=87", "after=88"),
            events,
            qtest,
            raw,
        )

    def test_panic(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console + "\n[panic] at kernel/src/entry/mod.rs:1",
            events,
            qtest,
            raw,
        )

    def test_timeout_and_nonzero_raw_exit(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console,
            events,
            qtest.replace("... ok", "... FAILED") + "\ntimed out after 20s",
            101,
        )

    def test_empty_evidence(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, [], qtest, raw)

    def test_truncated_diagnose_line(self):
        console, events, qtest, raw = valid_evidence()
        with tempfile.TemporaryDirectory() as at:
            path = Path(at) / "truncated.jsonl"
            path.write_text("\n".join(daemon_rtc.json.dumps(event) for event in events[:-1]) + '\n{"h":1')
            self.assertRaises(AssertionError, daemon_rtc.events_of, path)

    def test_missing_halt_record(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, events[:-1], qtest, raw)

    def test_missing_actual_daemon_birth(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, events[:1] + events[2:], qtest, raw)

    def test_daemon_dies_before_replacement_spawn(self):
        console, events, qtest, raw = valid_evidence()
        events[6]["when"] = 2900
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_provider_closes_before_daemon(self):
        console, events, qtest, raw = valid_evidence()
        daemon = "system: retired name=daemon task=18 team=22"
        rtc = "system: retired name=rtc task=31 team=40"
        console = console.replace(daemon + "\n" + rtc, rtc + "\n" + daemon)
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_controller_reap_missing(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, events[:5] + events[6:], qtest, raw)


if __name__ == "__main__":
    unittest.main()
