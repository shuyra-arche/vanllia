"""The coordinator owns the actual serial helper process, without a VM."""

import json
from pathlib import Path
import time

from tests.support.boot import BootCase


class HelperLifecycle(BootCase):
    def test_recorded_feed_pid_is_actual_helper_and_exits_after_run(self):
        self.prepare_qtest()
        evidence = self.tmp / 'helper.json'
        self.env['STUB_HELPER_EVIDENCE'] = str(evidence)
        cargo = self.bin / 'cargo'
        stub = cargo.read_text()
        marker = 'if a[:1] == ["qtest"]:\n'
        self.assertEqual(stub.count(marker), 1)
        stub = stub.replace(marker, marker + '''    pid = int(Path(os.environ["FEED_PID"]).read_text())
    command = Path(f"/proc/{pid}/cmdline").read_bytes().decode().split("\\0")
    Path(os.environ["STUB_HELPER_EVIDENCE"]).write_text(json.dumps({"pid":pid,"command":command}))
''')
        cargo.write_text(stub)
        result = self.invoke('qtest.nu', '--scene', 'product', '--feed-after', '0')
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        owned = json.loads(evidence.read_text())
        self.assertIn('acceptance.transport.feed', owned['command'])
        deadline = time.monotonic() + 2
        command = Path(f"/proc/{owned['pid']}/cmdline")
        while command.exists() and b'acceptance.transport.feed' in command.read_bytes():
            self.assertLess(time.monotonic(), deadline, 'feed helper survived coordinator cleanup')
            time.sleep(.01)

    def test_invalid_driver_or_timing_fails_before_any_external_command(self):
        for args in [('--feed-driver', 'tests.fixtures.bad'),
                     ('--feed-after', '-1'),
                     ('--feed-driver', 'acceptance.drivers.shell.smoke', '--feed-script', 'user.py')]:
            with self.subTest(args=args):
                result = self.invoke('qtest.nu', '--scene', 'product', *args)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.calls(), [])
