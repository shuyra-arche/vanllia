"""Runtime evidence location is an input, independent of repository layout."""

from pathlib import Path
import socket
import tempfile
import unittest
from unittest.mock import patch

from acceptance.drivers.memory_transfer import fresh, race, tlb
from acceptance.drivers.memory_transfer.harness import Harness


class DriverContext(unittest.TestCase):
    def test_missing_or_relative_diagnostic_input_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            unused, peer = socket.socketpair()
            with unused, peer:
                for driver in (fresh, race, tlb):
                    for diagnose in ('', 'relative.jsonl'):
                        with self.subTest(driver=driver.__name__, diagnose=diagnose), patch.dict('os.environ', {
                            'SCENE_SUMMARY_PENDING': str(Path(directory) / 'pending.json'),
                            'VANLLIA_DIAGNOSE': diagnose,
                        }, clear=True):
                            with self.assertRaisesRegex(AssertionError, 'VANLLIA_DIAGNOSE'):
                                driver.run(unused, str(Path(directory) / 'capture'))
                with patch.dict('os.environ', {}, clear=True):
                    with self.assertRaisesRegex(AssertionError, 'VANLLIA_DIAGNOSE'):
                        Harness(unused, str(Path(directory) / 'capture'))
