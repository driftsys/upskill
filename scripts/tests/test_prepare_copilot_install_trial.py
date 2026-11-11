"""Offline tests for the Copilot installation trial preparation script."""
import hashlib
import json
from pathlib import Path
import platform
import stat
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / 'prepare_copilot_install_trial.py'
VERSION = '9.8.7'
GOOD_NAME = 'upskill-' + 'a' * 39 + '-' + '0' * 16


def digest(data):
    return hashlib.sha256(data).hexdigest()


class PrepareTrial(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.parent_dir = self.root / 'parents'
        self.parent_dir.mkdir()
        self.markers = self.root / 'markers'
        self.markers.mkdir()
        self.env_record = self.root / 'env.json'

    def make_pin(self, name, content):
        path = self.root / name
        path.write_text(
            f'#!/bin/sh\ntouch {self.markers / name}\n' + content)
        path.chmod(0o755)
        return path

    def make_fixture(self, *, files=9, commands=11, native_name=GOOD_NAME, fail=False):
        path = self.root / 'fixture'
        body = (
            f'#!{sys.executable}\n'
            'import json, os, sys\n'
            'from pathlib import Path\n'
            f'Path({str(self.env_record)!r}).write_text(json.dumps('
            '{"env": dict(os.environ), "argv": sys.argv[1:]}))\n'
            f'if {fail!r}:\n'
            '    sys.exit(3)\n'
            'root = Path(sys.argv[2])\n'
            '(root / "sub").mkdir(parents=True)\n'
            f'plan = {{"commands": list(range({commands})), "native_name": {native_name!r}}}\n'
            '(root / "plan.json").write_text(json.dumps(plan))\n'
            f'for i in range({files} - 1):\n'
            '    (root / "sub" / f"f{i}.txt").write_text(f"file {i}")\n')
        path.write_text(body)
        path.chmod(0o755)
        return path

    def run_script(self, fixture, **overrides):
        launcher = self.make_pin('launcher', 'echo launcher\n')
        node = self.make_pin('node', 'echo node\n')
        entry = self.make_pin('entrypoint', 'echo entry\n')
        wrapper = self.root / 'package.json'
        wrapper.write_text(json.dumps({'name': 'wrapper', 'version': VERSION}))
        args = [
            sys.executable, str(SCRIPT),
            '--fixture-binary', str(fixture),
            '--required-cli-version', '1.2.3',
            '--parent-dir', str(self.parent_dir),
            '--launcher', str(launcher),
            '--node', str(node),
            '--entrypoint', str(entry),
            '--wrapper-package', str(wrapper),
        ]
        return subprocess.run(args, capture_output=True, text=True)

    def prepared(self):
        result = self.run_script(self.make_fixture())
        self.assertEqual(result.returncode, 0, result.stderr)
        out = json.loads(result.stdout)
        parent = Path(out['packet_root']).parent
        return result, out, parent

    def test_preparation_fields_and_values(self):
        result, out, parent = self.prepared()
        prep_bytes = (parent / 'preparation.json').read_bytes()
        prep = json.loads(prep_bytes)
        self.assertEqual(prep_bytes, (json.dumps(prep, indent=2) + '\n').encode())
        self.assertEqual(list(prep), [
            'status', 'os', 'machine', 'launcher', 'node', 'platform_entrypoint',
            'wrapper_package_version', 'required_cli_version', 'version_mismatch_policy',
            'plan_sha256', 'fixture_files'])
        self.assertEqual(prep['status'], 'prepared only; no native command executed')
        self.assertEqual(prep['os'], platform.system())
        self.assertEqual(prep['machine'], platform.machine())
        self.assertEqual(prep['wrapper_package_version'], VERSION)
        self.assertEqual(prep['required_cli_version'], '1.2.3')
        self.assertEqual(
            prep['version_mismatch_policy'],
            'stop before inventories or mutations; no update or fallback')
        for key, name in (('launcher', 'launcher'), ('node', 'node'),
                          ('platform_entrypoint', 'entrypoint')):
            path = self.root / name
            data = path.read_bytes()
            self.assertEqual(prep[key], {
                'selected': str(path), 'resolved': str(path.resolve()),
                'sha256': digest(data), 'bytes': len(data),
                'mode': oct(stat.S_IMODE(path.stat().st_mode))})
        packet = parent / 'packet'
        plan_hash = digest((packet / 'plan.json').read_bytes())
        self.assertEqual(prep['plan_sha256'], plan_hash)
        files = prep['fixture_files']
        self.assertEqual(len(files), 9)
        self.assertEqual([f['path'] for f in files],
                         [str(p.relative_to(packet)) for p in packet.rglob('*') if p.is_file()])
        for entry in files:
            path = packet / entry['path']
            data = path.read_bytes()
            self.assertEqual(entry, {
                'path': entry['path'], 'sha256': digest(data), 'bytes': len(data),
                'mode': oct(stat.S_IMODE(path.stat().st_mode))})
        self.assertEqual(out, {
            'packet_root': str(packet), 'plan_sha256': plan_hash,
            'preparation_sha256': digest(prep_bytes), 'native_name': GOOD_NAME})
        self.assertEqual(result.stdout, json.dumps(out, indent=2) + '\n')

    def test_modes(self):
        _, _, parent = self.prepared()
        self.assertTrue(parent.name.startswith('upskill-copilot-install-trial-'))
        self.assertEqual(parent.parent, self.parent_dir)
        mode = lambda p: stat.S_IMODE(p.stat().st_mode)
        self.assertEqual(mode(parent), 0o700)
        self.assertEqual(mode(parent / 'generator-home'), 0o700)
        self.assertEqual(mode(parent / 'preparation.json'), 0o600)

    def test_fixture_receives_exact_environment(self):
        _, _, parent = self.prepared()
        record = json.loads(self.env_record.read_text())
        home = str(parent / 'generator-home')
        # The fake fixture's Python interpreter adds these at its own startup.
        for added_by_interpreter in ('__CF_USER_TEXT_ENCODING', 'LC_CTYPE'):
            record['env'].pop(added_by_interpreter, None)
        self.assertEqual(record['env'], {
            'HOME': home, 'USERPROFILE': home, 'PATH': '/usr/bin:/bin'})
        self.assertEqual(record['argv'], ['--copilot', str(parent / 'packet')])

    def test_count_checks_fail(self):
        for kwargs in ({'files': 8}, {'commands': 10}, {'native_name': 'upskill-' + '0' * 64}):
            with self.subTest(**kwargs):
                result = self.run_script(self.make_fixture(**kwargs))
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, '')
                self.assertIn(next(iter(kwargs)).split('_')[0], result.stderr)
                self.assertNotIn('Traceback', result.stderr)

    def test_fixture_failure_propagates(self):
        result = self.run_script(self.make_fixture(fail=True))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, '')

    def test_pinned_programs_are_never_executed(self):
        self.prepared()
        self.assertEqual(list(self.markers.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
