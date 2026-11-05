"""Parser negative controls: each case mutates otherwise valid runner evidence."""

import copy
import sys
import tempfile
import unittest
from pathlib import Path

from acceptance.oracles.daemon import stop as daemon_stop
from tests.fixtures.daemon.stop import valid_evidence




class DaemonStopVerifier(unittest.TestCase):
    def test_checked_entry_rejection_proof_is_bound_to_actual_root_and_controller(self):
        console, events, qtest, raw = valid_evidence()
        for old, new in (
            ("checked entry host=10", "checked entry host=99"),
            ("rejected=7", "rejected=6"),
            ("fake_owner=30", "fake_owner=99"),
            ("reclaimed=2", "reclaimed=1"),
        ):
            with self.subTest(field=old), self.assertRaises(AssertionError):
                self.verify(console.replace(old, new), events, qtest, raw)

    def test_publisher_identity_host_owner_and_mark_must_match(self):
        console, events, qtest, raw = valid_evidence()
        for old, new in (
            ("publisher task=17", "publisher task=99"),
            ("host=10 entry_owner", "host=99 entry_owner"),
            ("entry_owner=17", "entry_owner=99"),
            ("entry_mark=44", "entry_mark=45"),
            ("entry_mark=44 expected_mark=44", "entry_mark=0 expected_mark=0"),
        ):
            with self.subTest(field=old), self.assertRaises(AssertionError):
                self.verify(console.replace(old, new), events, qtest, raw)

    def test_publisher_revocation_must_cover_every_stable_check(self):
        console, events, qtest, raw = valid_evidence()
        for old, new in (
            ("revoked checks=12", "revoked checks=11"),
            ("revoked checks=12 host=10", "revoked checks=12 host=99"),
            ("host=10 target=17", "host=10 target=99"),
        ):
            with self.subTest(field=old), self.assertRaises(AssertionError):
                self.verify(console.replace(old, new), events, qtest, raw)

    def verify(self, console, events, qtest, raw=0):
        return daemon_stop.verify(console, events, qtest, raw)

    def test_valid_evidence_passes(self):
        self.verify(*valid_evidence())

    def test_short_observation_window_is_rejected(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError, self.verify,
            console.replace("elapsed_ns=650000000", "elapsed_ns=100000000"),
            events, qtest, raw,
        )

    def test_outsider_faces_must_belong_to_actual_root(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError, self.verify,
            console.replace("observe=10 recover=10", "observe=99 recover=99"),
            events, qtest, raw,
        )

    def test_outsider_source_is_not_controller(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("outsider source=30", "outsider source=99"),
            events,
            qtest,
            raw,
        )

    def test_outsider_not_denied(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("denied=Denied", "denied=OK"),
            events,
            qtest,
            raw,
        )

    def test_merge_is_not_reaper_origin(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("pending_origin=Reaper", "pending_origin=External"),
            events,
            qtest,
            raw,
        )

    def test_merge_sender_is_not_actor(self):
        console, events, qtest, raw = valid_evidence()
        console = console.replace(
            "lifecycle received target=fixturetarget sender=29",
            "lifecycle received target=fixturetarget sender=99",
        )
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_intent_not_stopped(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("intent target=fixturetarget run=false", "intent target=fixturetarget run=true"),
            events,
            qtest,
            raw,
        )

    def test_missing_actor_reply(self):
        console, events, qtest, raw = valid_evidence()
        console = "\n".join(
            line for line in console.splitlines() if "daemon-stop-actor: target=fixturetarget reply=ok" not in line
        )
        self.assertRaises(AssertionError, self.verify, console, events, qtest, raw)

    def test_actor_state_is_not_dead(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("state=Dead", "state=Ready"),
            events,
            qtest,
            raw,
        )

    def test_missing_root_ack(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("stop-root: ack controller=30", "stop-root: ack controller=99"),
            events,
            qtest,
            raw,
        )

    def test_namespace_reappears(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("missing=12", "missing=11"),
            events,
            qtest,
            raw,
        )

    def test_too_few_stable_checks(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console.replace("checks=12 missing=12", "checks=1 missing=1"),
            events,
            qtest,
            raw,
        )

    def test_second_target_construction(self):
        console, events, qtest, raw = valid_evidence()
        extra = "system: static construct name=fixturetarget task=31 team=40 owner=10"
        self.assertRaises(
            AssertionError, self.verify, console + "\n" + extra, events, qtest, raw
        )

    def test_recovery_after_stop(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console + "\nsystem: recover admitted target=fixturetarget caller=18",
            events,
            qtest,
            raw,
        )

    def test_replacement_observed(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(
            AssertionError,
            self.verify,
            console + "\nstop-root: new task=31 team=40",
            events,
            qtest,
            raw,
        )

    def test_old_helper_never_reaped(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, events[:6] + events[7:], qtest, raw)

    def test_daemon_dies_before_old_reap(self):
        console, events, qtest, raw = valid_evidence()
        events = copy.deepcopy(events)
        events[9]["when"] = 2500
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
            path.write_text(
                "\n".join(daemon_stop.json.dumps(event) for event in events[:-1])
                + '\n{"h":1'
            )
            self.assertRaises(AssertionError, daemon_stop.events_of, path)

    def test_missing_halt_record(self):
        console, events, qtest, raw = valid_evidence()
        self.assertRaises(AssertionError, self.verify, console, events[:-1], qtest, raw)


if __name__ == "__main__":
    unittest.main()
