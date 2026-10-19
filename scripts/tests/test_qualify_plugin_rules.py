"""Offline mechanics only; these tests cannot qualify a native client."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


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
        for key, value in [('sha256', 'edited'), ('mode', 420)]:
            changed = self.observation([], {})
            changed['team'][key] = value
            self.assertEqual(self.assess(control=changed), 'team_changed')

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
            package = Path(fixture['packages'][0])
            result = subprocess.run([sys.executable, str(package / 'hook.py')],
                input=json.dumps({'cwd': fixture['project'], 'session_id': 'test-session',
                                  'source': 'startup', 'hook_event_name': 'SessionStart'}),
                text=True, capture_output=True, check=True)
            context = json.loads(result.stdout)['hookSpecificOutput']['additionalContext']
            self.assertIn(fixture['rule_markers'][0], context)
            self.assertNotIn(fixture['rule_markers'][1], context)
            self.assertEqual((team.read_bytes(), team.stat().st_mode & 0o777), before)
            trace = [json.loads(x) for x in Path(fixture['trace']).read_text().splitlines()]
            self.assertEqual([(x['rule'], x['source']) for x in trace], [('a', 'startup')])

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
                            'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME', 'OPENCODE_CONFIG'):
                    self.assertTrue(Path(env[key]).is_relative_to(Path(fixture['root'])), key)
                self.assertFalse(any(p.name in ('auth.json', 'credentials.json') for p in Path(fixture['root']).rglob('*')))
                self.assertFalse(any(k.startswith('GIT_') or k.endswith('TOKEN') or k.endswith('API_KEY') for k in env))


if __name__ == '__main__':
    unittest.main()
