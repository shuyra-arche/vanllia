"""Serial listener tests use local sockets and synthetic user extensions."""

from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import unittest

from acceptance.transport.feed import load_driver
from tests.support.paths import ROOT


class FeedTransport(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.port = self.root / 'listener.port'
        self.capture = self.root / 'serial.log'

    def start(self, *extra):
        process = subprocess.Popen([sys.executable, '-B', '-m', 'acceptance.transport.feed',
            str(self.port), str(self.capture), '\x04', '0', *extra],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        def finish():
            if process.poll() is None:
                process.terminate()
            process.communicate(timeout=5)
        self.addCleanup(finish)
        deadline = time.monotonic() + 5
        while not self.port.exists():
            self.assertIsNone(process.poll(), process.communicate() if process.poll() is not None else '')
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.01)
        return process

    def test_builtin_uses_normal_package_identity(self):
        from acceptance.drivers.shell import smoke
        self.assertIs(load_driver('acceptance.drivers.shell.smoke'), smoke)

    def test_invalid_namespace_and_conflicting_inputs_are_rejected(self):
        with self.assertRaises(ValueError):
            load_driver('tests.fixtures.something')
        with self.assertRaises(ValueError):
            load_driver('acceptance.drivers.shell.smoke', 'external.py')

    def test_explicit_external_extension_keeps_its_relative_imports(self):
        (self.root / 'value.py').write_text('VALUE = 17\n')
        script = self.root / 'driver.py'
        script.write_text('from .value import VALUE\ndef run(conn, capture): return VALUE\n')
        driver = load_driver(script=str(script))
        self.assertEqual(driver.run(None, None), 17)

    def test_default_feed_delivers_eof_and_captures_peer_bytes(self):
        self.start()
        with socket.create_connection(('127.0.0.1', int(self.port.read_text())), timeout=2) as conn:
            received = b''
            while len(received) < 2:
                received += conn.recv(2 - len(received))
            self.assertEqual(received, b'\x04\n')
            conn.sendall(b'local synthetic peer\n')
            deadline = time.monotonic() + 2
            while b'local synthetic peer' not in self.capture.read_bytes():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.01)

    def test_driver_failure_never_publishes_ok(self):
        script = self.root / 'reject.py'
        script.write_text('def run(conn, capture): raise AssertionError("synthetic rejection")\n')
        process = self.start('--script', str(script))
        with socket.create_connection(('127.0.0.1', int(self.port.read_text())), timeout=2):
            process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertEqual(Path(str(self.capture) + '.result').read_text(), 'synthetic rejection')

    def test_external_module_state_is_fresh_for_each_connection(self):
        script = self.root / 'counter.py'
        script.write_text('calls = 0\ndef run(conn, capture):\n    global calls\n    calls += 1\n    conn.sendall(str(calls).encode())\n')
        self.start('--script', str(script))
        for _ in range(2):
            with socket.create_connection(('127.0.0.1', int(self.port.read_text())), timeout=2) as conn:
                self.assertEqual(conn.recv(1), b'1')
