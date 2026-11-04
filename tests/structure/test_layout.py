"""Import and discovery contracts for the acceptance owners."""

import ast
from collections import Counter
import importlib
import hashlib
import inspect
import json
import pkgutil
import sys
import textwrap
import unittest

from tests.support.paths import ROOT


def import_errors(module, source):
    """Check dependency direction, including relative imports and loader writes."""
    errors = []
    tree = ast.parse(source)
    owners = {}
    for function in ast.walk(tree):
        if isinstance(function, (ast.FunctionDef, ast.AsyncFunctionDef)):
            for child in ast.walk(function):
                owners[id(child)] = function.name
    for node in ast.walk(tree):
        targets = []
        if isinstance(node, ast.Import):
            targets = [alias.name for alias in node.names]
        elif isinstance(node, ast.ImportFrom):
            if node.level:
                package = module.rsplit('.', 1)[0]
                targets = [importlib.util.resolve_name('.' * node.level + (node.module or ''), package)]
            else:
                targets = [node.module or '']
            targets += [targets[0] + '.' + alias.name for alias in node.names]
        for target in targets:
            if target == 'trace' and module != 'tests.structure.test_layout':
                errors.append('ambiguous trace import')
            if module.startswith('acceptance.') and target.startswith('tests'):
                errors.append('runtime depends on host tests')
            if module.startswith('acceptance.oracles.') and target.startswith((
                    'acceptance.drivers', 'acceptance.fixtures', 'acceptance.transport', 'importlib')):
                errors.append('oracle depends on execution')
            if module.startswith('acceptance.contracts.') and target.startswith('acceptance'):
                errors.append('value contract depends on runtime')
            if module.startswith('tests.fixtures.') and target.startswith(('acceptance', 'tests.support')):
                errors.append('synthetic fixture depends on implementation')
        if isinstance(node, ast.Attribute) and ast.unparse(node).startswith('sys.path'):
            errors.append('sys.path mutation or dependency')
        if isinstance(node, ast.Name) and node.id == '__file__' and module.startswith('acceptance.'):
            errors.append('runtime infers paths from source location')
        if module.startswith('acceptance.oracles.') and isinstance(node, ast.Attribute) and ast.unparse(node).startswith('importlib.'):
            errors.append('oracle uses dynamic execution imports')
        if isinstance(node, ast.Subscript) and isinstance(node.ctx, ast.Store) and ast.unparse(node.value) == 'sys.modules':
            if not (module == 'acceptance.transport.feed' and owners.get(id(node)) == 'load_driver'):
                errors.append('internal module registry injection')
    return errors


def cases(suite):
    for value in suite:
        if isinstance(value, unittest.TestSuite):
            yield from cases(value)
        else:
            yield value


def assertion_fingerprint(source):
    tree = ast.parse(textwrap.dedent(source))
    method = next(node for node in tree.body if isinstance(node, ast.FunctionDef))
    assertions = [ast.dump(node, include_attributes=False) for node in ast.walk(method)
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
        and isinstance(node.func.value, ast.Name) and node.func.value.id == 'self'
        and node.func.attr.startswith('assert')]
    return hashlib.sha256(json.dumps(assertions).encode()).hexdigest()


class Layout(unittest.TestCase):
    def test_every_original_case_is_discovered_once(self):
        baseline = json.loads((ROOT / 'tests/coverage-baseline.json').read_text())
        expected = [item['case'] for item in baseline]
        self.assertEqual(len(expected), 305)
        self.assertEqual(len(set(expected)), 305)
        suite = unittest.defaultTestLoader.discover(str(ROOT / 'tests'), top_level_dir=str(ROOT))
        fingerprints = json.loads((ROOT / 'tests/assertion-baseline.json').read_text())
        self.assertEqual(fingerprints['base'], 'ff2c059c7df1545afedb95eb74dce71f40ca3f8d')
        self.assertEqual(set(fingerprints['cases']), set(expected))
        found = Counter()
        for case in cases(suite):
            owner, method = case.id().split('.')[-2:]
            if owner in {'BuildEntry', 'BoardEntry', 'RunnerEntry', 'QtestEntry'}:
                owner = 'Entry'
            identity = owner + '.' + method
            found[identity] += 1
            if identity in fingerprints['cases']:
                actual = assertion_fingerprint(inspect.getsource(getattr(type(case), method)))
                self.assertEqual(actual, fingerprints['cases'][identity], identity)
        self.assertEqual({name: found[name] for name in expected}, dict.fromkeys(expected, 1))
        self.assertNotEqual(assertion_fingerprint('def test_example(self): self.assertEqual(1, 2)'),
                            assertion_fingerprint('def test_example(self): pass'))

    def test_actual_import_closure_respects_ownership(self):
        for owner in ('acceptance', 'tests'):
            for path in (ROOT / owner).rglob('*.py'):
                module = '.'.join(path.relative_to(ROOT).with_suffix('').parts)
                self.assertEqual(import_errors(module, path.read_text()), [], module)

    def test_boundary_checker_rejects_reverse_edges_and_loaders(self):
        bad = [
            ('acceptance.oracles.memory_transfer.fresh', 'from acceptance.drivers import memory_transfer'),
            ('acceptance.oracles.memory_transfer.fresh', 'from acceptance import drivers'),
            ('acceptance.oracles.device_recovery.ruin', 'from ...drivers import device_recovery'),
            ('acceptance.drivers.shell.smoke', 'from tests.fixtures import daemon'),
            ('acceptance.oracles.device_recovery.execution', 'from trace import check'),
            ('tests.oracles.daemon.rtc', 'import sys\nsys.path.insert(0, "old")'),
            ('tests.oracles.memory_transfer.race', 'import sys\nsys.modules["fake"] = object()'),
            ('tests.fixtures.daemon.rtc', 'from acceptance.oracles.daemon import rtc'),
            ('acceptance.oracles.daemon.rtc', 'importlib.import_module("acceptance.drivers.shell.smoke")'),
        ]
        for module, source in bad:
            with self.subTest(module=module, source=source):
                self.assertTrue(import_errors(module, source))

    def test_boundary_checker_accepts_forward_edges(self):
        self.assertEqual(import_errors('acceptance.drivers.memory_transfer.fresh',
            'from acceptance.oracles.memory_transfer.fresh import judge'), [])
        self.assertEqual(import_errors('acceptance.oracles.memory_transfer.fresh',
            'from acceptance.evidence.kernel_events import room_payload'), [])

    def test_runtime_imports_do_not_replace_stdlib_trace(self):
        import trace
        original = sys.modules['trace']
        self.assertTrue(hasattr(trace, 'Trace'))
        import acceptance
        for module in pkgutil.walk_packages(acceptance.__path__, acceptance.__name__ + '.'):
            importlib.import_module(module.name)
        self.assertIs(sys.modules['trace'], original)

    def test_old_layout_and_duplicate_loader_names_are_absent(self):
        self.assertEqual(list((ROOT / 'acceptance/tests').rglob('*.py')), [])
        self.assertEqual(list((ROOT / 'scripts/tests').rglob('*.py')), [])
        self.assertEqual(list((ROOT / 'acceptance').rglob('trace.py')), [])
        self.assertFalse(any('-' in path.name for path in (ROOT / 'tests').rglob('*') if path.is_dir()))
