"""Offline mechanics only; these tests cannot qualify a native client."""
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).parents[1] / 'qualify-plugin-rules.py'


class PluginRuleTests(unittest.TestCase):
    def setUp(self):
        if not SCRIPT.exists():
            self.fail('Independent plugin qualification harness is not implemented')
        spec = importlib.util.spec_from_file_location('plugin_rules', SCRIPT)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)

    def observation(self, rules, counts):
        return {'state': 'success', 'completed': True, 'tools_observed': False,
                'version': '0.163.0', 'response': {'team': 'TEAM', 'rules': rules},
                'activation_counts': counts, 'team': {'sha256': 'original', 'mode': 416}}

    def assess(self, positive=None, control=None, on=None, off=None):
        return self.q.classify_pair(
            positive if positive is not None else self.observation(['A', 'B'], {'a': 1, 'b': 1}),
            control if control is not None else self.observation([], {}),
            expected_positive={'team': 'TEAM', 'rules': ['A', 'B']},
            expected_control={'team': 'TEAM', 'rules': []},
            positive_counts=on if on is not None else {'a': 1, 'b': 1},
            control_counts=off if off is not None else {},
            team_before={'sha256': 'original', 'mode': 416}, version='0.163.0')

    def test_team_instructions_unchanged(self):
        self.assertEqual(self.assess(), 'observed')
        for side in ('positive', 'control'):
            for key, value in [('sha256', 'edited'), ('mode', 420)]:
                changed = self.observation(['A', 'B'], {'a': 1, 'b': 1}) if side == 'positive' else self.observation([], {})
                changed['team'][key] = value
                self.assertEqual(self.assess(**{side: changed}), 'team_changed')

    def test_positive_requires_the_team_and_each_independent_rule(self):
        for response in ({'team': None, 'rules': ['A', 'B']},
                         {'team': 'TEAM', 'rules': ['A']},
                         {'team': 'TEAM', 'rules': ['A', 'C']},
                         {'team': 'TEAM', 'rules': ['A', 'A', 'B']}):
            value = self.observation(['A', 'B'], {'a': 1, 'b': 1})
            value['response'] = response
            self.assertEqual(self.assess(positive=value), 'positive_response_mismatch')
        for field in ('response', 'team'):
            value = self.observation(['A', 'B'], {'a': 1, 'b': 1})
            del value[field]
            self.assertNotEqual(self.assess(positive=value), 'observed')

    def test_failed_positive_cannot_qualify_negative(self):
        for state in ('failure', 'timeout', 'authentication_failure', 'unavailable'):
            positive = self.observation(['A', 'B'], {'a': 1, 'b': 1})
            positive['state'] = state
            self.assertEqual(self.assess(positive=positive), 'positive_control_failed')
        for field, value in [('completed', False), ('tools_observed', True), ('version', '0.162.0')]:
            positive = self.observation(['A', 'B'], {'a': 1, 'b': 1})
            positive[field] = value
            self.assertEqual(self.assess(positive=positive), 'positive_control_failed')

    def test_disabled_plugin_retains_team_guidance(self):
        self.assertEqual(self.assess(), 'observed')
        missing = self.observation([], {})
        missing['response']['team'] = None
        self.assertEqual(self.assess(control=missing), 'control_response_mismatch')
        failed = self.observation([], {})
        failed['state'] = 'authentication_failure'
        self.assertEqual(self.assess(control=failed), 'control_failed')
        for field, value in [('completed', False), ('tools_observed', True), ('version', '0.162.0')]:
            control = self.observation([], {})
            control[field] = value
            self.assertEqual(self.assess(control=control), 'control_failed')

    def test_unintended_reader_never_activates_payload(self):
        self.assertEqual(self.assess(control=self.observation(['A'], {'a': 1})), 'control_response_mismatch')
        self.assertEqual(self.assess(control=self.observation([], {'a': 1})), 'activation_count_mismatch')
        self.assertEqual(self.assess(), 'observed')

    def test_direct_and_packaged_rule_activate_once(self):
        self.assertEqual(self.assess(), 'observed')
        duplicated = self.observation(['A', 'B'], {'a': 2, 'b': 1})
        self.assertEqual(self.assess(positive=duplicated), 'activation_count_mismatch')
        missing = self.observation(['A', 'B'], {})
        self.assertEqual(self.assess(positive=missing), 'activation_count_mismatch')
        for counts in ({'a': 1, 'b': 2}, {'a': 1, 'other': 1}, {'other': 1, 'b': 1},
                       {'a': 1, 'b': 1, 'other': 1}, {'a': 2, 'b': 1}):
            self.assertEqual(self.assess(positive=self.observation(['A', 'B'], counts)), 'activation_count_mismatch')
        # Distinguishable counts pin the association between rule ID and count.
        self.assertEqual(self.assess(positive=self.observation(['A', 'B'], {'b': 2, 'a': 1}),
                                     on={'a': 1, 'b': 2}), 'observed')
        self.assertEqual(self.assess(positive=self.observation(['A', 'B'], {'a': 2, 'b': 1}),
                                     on={'a': 1, 'b': 2}), 'activation_count_mismatch')

    def test_noninteger_activation_evidence_cannot_pass(self):
        for count in (True, 1.0, '1'):
            value = self.observation(['A', 'B'], {'a': count, 'b': 1})
            self.assertEqual(self.assess(positive=value), 'activation_count_mismatch')

    def test_missing_observations_fail_closed(self):
        for field in self.observation([], {}):
            value = self.observation([], {})
            del value[field]
            self.assertNotEqual(self.assess(control=value), 'observed', field)

    def test_fixture_hook_uses_independent_payload_and_preserves_team(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = self.q.prepare(Path(tmp) / 'fixture', 'codex')
            team = Path(fixture['project']) / 'AGENTS.md'
            before = (team.read_bytes(), team.stat().st_mode & 0o777)
            self.assertIn(fixture['team_marker'], team.read_text())
            self.assertEqual(len(set(fixture['rule_markers'])), 2)
            for marker in fixture['rule_markers']:
                self.assertNotIn(marker, team.read_text())
            for i, package in enumerate(fixture['packages']):
                result = subprocess.run([sys.executable, str(Path(package) / 'hook.py')],
                    input=json.dumps({'cwd': fixture['project'], 'session_id': 'test-session',
                                      'source': 'startup', 'hook_event_name': 'SessionStart'}),
                    text=True, capture_output=True, check=True)
                context = json.loads(result.stdout)['hookSpecificOutput']['additionalContext']
                self.assertEqual(context.count(fixture['rule_markers'][i]), 1)
                self.assertNotIn(fixture['rule_markers'][1-i], context)
            self.assertEqual((team.read_bytes(), team.stat().st_mode & 0o777), before)
            trace = [json.loads(x) for x in Path(fixture['trace']).read_text().splitlines()]
            self.assertEqual(trace, [{'rule': 'a', 'session': 'test-session', 'source': 'startup'},
                                     {'rule': 'b', 'session': 'test-session', 'source': 'startup'}])

    def test_fixture_hook_refuses_other_directory_or_event(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = self.q.prepare(Path(tmp) / 'fixture', 'codex')
            hook = Path(fixture['packages'][0]) / 'hook.py'
            for event in ({'cwd': tmp, 'hook_event_name': 'SessionStart', 'source': 'startup', 'session_id': 'x'},
                          {'cwd': fixture['project'], 'hook_event_name': 'PreToolUse', 'source': 'startup', 'session_id': 'x'}):
                result = subprocess.run([sys.executable, str(hook)], input=json.dumps(event),
                    text=True, capture_output=True, check=True)
                self.assertEqual(json.loads(result.stdout), {})
            self.assertFalse(Path(fixture['trace']).exists())

    def test_hook_rejects_unknown_sources_and_invalid_sessions(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = self.q.prepare(Path(tmp) / 'fixture', 'codex')
            hook = Path(fixture['packages'][0]) / 'hook.py'
            for changes in ({'source': 'unknown'}, {'source': None}, {'session_id': 1}, {'session_id': None}):
                event = {'cwd': fixture['project'], 'hook_event_name': 'SessionStart', 'source': 'startup', 'session_id': 'session'}
                event.update(changes)
                result = subprocess.run([sys.executable, str(hook)], input=json.dumps(event), text=True, capture_output=True, check=True)
                self.assertEqual(json.loads(result.stdout), {})
            self.assertFalse(Path(fixture['trace']).exists())

    def test_hook_records_each_supported_lifecycle_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = self.q.prepare(Path(tmp) / 'fixture', 'codex')
            hook = Path(fixture['packages'][0]) / 'hook.py'
            for source in ('resume', 'clear', 'compact', 'fork'):
                event = {'cwd': fixture['project'], 'hook_event_name': 'SessionStart', 'source': source, 'session_id': source+'-session'}
                result = subprocess.run([sys.executable, str(hook)], input=json.dumps(event), text=True, capture_output=True, check=True)
                self.assertIn(fixture['rule_markers'][0], json.loads(result.stdout)['hookSpecificOutput']['additionalContext'])
            records = [json.loads(x) for x in Path(fixture['trace']).read_text().splitlines()]
            self.assertEqual(records, [{'rule': 'a', 'session': s+'-session', 'source': s} for s in ('resume','clear','compact','fork')])

    def test_snapshot_detects_real_bytes_and_modes(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'team'
            path.write_bytes(b'abc')
            path.chmod(0o640)
            original = self.q.snapshot(path)
            self.assertEqual(original, {'sha256': 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad', 'mode': 0o640})
            path.write_bytes(b'abcd')
            self.assertNotEqual(self.q.snapshot(path)['sha256'], original['sha256'])
            path.chmod(0o600)
            self.assertEqual(self.q.snapshot(path)['mode'], 0o600)

    def test_prepare_rejects_repository_descendant_before_creation(self):
        path = Path(__file__).resolve().parents[2] / ('refused-probe-' + secrets.token_hex(16))
        try:
            with self.assertRaises(ValueError):
                self.q.prepare(path, 'codex')
            self.assertFalse(path.exists())
        finally:
            if path.exists():
                shutil.rmtree(path)

    def test_prepared_descriptors_never_claim_qualification(self):
        with tempfile.TemporaryDirectory() as tmp:
            for client in ('codex', 'opencode'):
                fixture = self.q.prepare(Path(tmp) / client, client)
                for descriptor in (fixture, json.loads((Path(fixture['root']) / 'fixture.json').read_text())):
                    self.assertIs(descriptor['qualified'], False)
                    self.assertEqual(descriptor['status'], 'prepared-only')
                text = (Path(fixture['project']) / 'AGENTS.md').read_text()
                self.assertIn(fixture['team_marker'], text)
                for i, package in enumerate(fixture['packages']):
                    self.assertNotIn(fixture['rule_markers'][i], text)
                    payload = (Path(package) / 'rule.txt').read_text()
                    self.assertIn(fixture['rule_markers'][i], payload)
                    self.assertNotIn(fixture['rule_markers'][1-i], payload)

    def test_prepare_rejects_existing_root_without_changes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            sentinel = root / 'manual'
            sentinel.write_text('keep')
            with self.assertRaises(ValueError):
                self.q.prepare(root, 'codex')
            self.assertEqual(sentinel.read_text(), 'keep')
            self.assertEqual(list(root.iterdir()), [sentinel])

    def test_prepare_never_copies_auth_or_inherits_native_roots(self):
        with tempfile.TemporaryDirectory() as tmp:
            for client in ('codex', 'opencode'):
                fixture = self.q.prepare(Path(tmp) / client, client)
                env = self.q.fixture_environment(Path(fixture['root']))
                for key in ('HOME', 'CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'COPILOT_HOME',
                            'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME'):
                    self.assertTrue(Path(env[key]).is_relative_to(Path(fixture['root'])), key)
                self.assertFalse(any(p.name in ('auth.json', 'credentials.json') for p in Path(fixture['root']).rglob('*')))
                self.assertFalse(any(k.startswith('GIT_') or k.endswith('TOKEN') or k.endswith('API_KEY') for k in env))

    def test_opencode_uses_isolated_xdg_without_a_directory_file_override(self):
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(os.environ, {'OPENCODE_CONFIG': '/unrelated/directory'}):
            fixture = self.q.prepare(Path(tmp) / 'fixture', 'opencode')
            env = self.q.fixture_environment(Path(fixture['root']))
            self.assertFalse('OPENCODE_CONFIG' in env, 'Native override must not point at a directory')
            self.assertTrue(Path(env['XDG_CONFIG_HOME']).is_dir())
            self.assertTrue(Path(env['XDG_CONFIG_HOME']).is_relative_to(Path(fixture['root'])))

    def test_hostile_ambient_auth_and_discovery_overrides_are_not_inherited(self):
        hostile = {key: '/unrelated/config' for key in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_CONFIG_COUNT',
            'CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'COPILOT_HOME', 'OPENCODE_CONFIG', 'OPENCODE_CONFIG_DIR',
            'OPENCODE_CONFIG_CONTENT', 'OPENCODE_DISABLE_PROJECT_CONFIG', 'OPENAI_API_KEY', 'ANTHROPIC_API_KEY',
            'GITHUB_TOKEN', 'GH_TOKEN', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME')}
        with tempfile.TemporaryDirectory() as tmp, mock.patch.dict(os.environ, hostile):
            env = self.q.fixture_environment(Path(tmp))
            self.assertTrue(all(value != '/unrelated/config' for value in env.values()))
            for key in ('GIT_DIR', 'GIT_CONFIG_COUNT', 'OPENCODE_CONFIG_CONTENT', 'OPENCODE_CONFIG_DIR',
                        'OPENCODE_DISABLE_PROJECT_CONFIG', 'OPENAI_API_KEY', 'ANTHROPIC_API_KEY', 'GITHUB_TOKEN', 'GH_TOKEN'):
                self.assertNotIn(key, env)


if __name__ == '__main__':
    unittest.main()
