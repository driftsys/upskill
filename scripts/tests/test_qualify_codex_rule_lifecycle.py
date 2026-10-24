"""Offline Q9 fixtures and payload classification; no native qualification."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import json
import subprocess
import sys
from unittest import mock


SCRIPT = Path(__file__).parents[1] / 'qualify-plugin-rules.py'


class CodexLifecycleTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('q9', SCRIPT)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)

    def test_fresh_fixture_has_two_independent_rules_without_team_guidance(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.prepare_codex_rule_lifecycle(Path(tmp) / 'fixture')
            self.assertEqual(len(f['packages']), 2)
            self.assertEqual(len(set(f['rule_bodies'].values())), 2)
            self.assertEqual(f['pin'], '0.164.0')
            market = json.loads((f['marketplace'] / '.agents/plugins/marketplace.json').read_text())
            self.assertEqual(market, {'name': f['market_name'], 'plugins': [
                {'name': p.name, 'source': {'source': 'local', 'path': './packages/' + p.name}}
                for p in f['packages']]})
            parents = subprocess.check_output(['git', '-C', str(f['project']), 'show', '-s', '--format=%P', 'HEAD'], text=True)
            self.assertEqual(parents.strip(), '')
            self.assertEqual(subprocess.check_output(['git', '-C', str(f['project']), 'ls-tree', '-r', '--name-only', 'HEAD']), b'')
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name, ('AGENTS.md', 'AGENTS.override.md', 'CLAUDE.md'))
            for (rule, body), package in zip(f['rule_bodies'].items(), f['packages']):
                self.assertEqual((package / 'rule.txt').read_text(), body)
                self.assertEqual(json.loads((package / '.codex-plugin/plugin.json').read_text()),
                                 {'name': package.name, 'version': '0.0.0', 'hooks': './hooks/hooks.json'})
                self.assertEqual(json.loads((package / 'hooks/hooks.json').read_text()),
                                 {'hooks': {'SessionStart': [{'hooks': [{'type': 'command', 'command': 'python3 "${PLUGIN_ROOT}/hook.py"'}]}]}})
                event = {'hook_event_name': 'SessionStart', 'source': 'startup',
                         'session_id': 'offline-session', 'cwd': str(f['project'])}
                result = subprocess.run([sys.executable, str(package / 'hook.py')],
                                        input=json.dumps(event), capture_output=True, text=True,
                                        env=self.q.fixture_environment(f['root']), check=True)
                self.assertEqual(json.loads(result.stdout), {'hookSpecificOutput': {
                    'hookEventName': 'SessionStart', 'additionalContext': body}})
                trace = list(map(json.loads, (f['root'] / 'activation.jsonl').read_text().splitlines()))
                self.assertEqual(trace[-1], {'rule': rule, 'session': 'offline-session', 'source': 'startup'})
            self.assertFalse(f['qualified'])

    def test_existing_root_and_discovery_roots_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for path in (root, *(root / p / 'fixture' for p in (
                    '.claude', '.github', '.codex', '.opencode', '.agents/skills', '.agents/plugins'))):
                with self.subTest(path=path), self.assertRaises(ValueError):
                    self.q.prepare_codex_rule_lifecycle(path)
            dangling = root / 'dangling'
            dangling.symlink_to(root / 'absent')
            with self.assertRaises(ValueError):
                self.q.prepare_codex_rule_lifecycle(dangling)
            with mock.patch.object(self.q, 'REPO', root.resolve() / 'repository'):
                with self.assertRaises(ValueError):
                    self.q.prepare_codex_rule_lifecycle(root / 'repository/fixture')

    def test_only_exact_developer_rule_payloads_count(self):
        bodies = {'a': 'RULE A\n', 'b': 'RULE B\n'}
        def message(role, text):
            return {'type': 'message', 'role': role, 'content': [{'type': 'input_text', 'text': text}]}
        messages = [message('developer', bodies['a']), message('developer', bodies['b']),
                    message('user', bodies['a']), message('assistant', bodies['b'])]
        self.assertEqual(self.q.count_developer_rule_bodies(messages, bodies), {'a': 1, 'b': 1})
        messages.append(message('developer', bodies['a']))
        self.assertEqual(self.q.count_developer_rule_bodies(messages, bodies), {'a': 2, 'b': 1})
        self.assertEqual(self.q.count_developer_rule_bodies([message('developer', bodies['a'] * 2)], bodies), {'a': 2, 'b': 0})
        other_block = message('developer', bodies['a'])
        other_block['content'][0]['type'] = 'input_image'
        other_record = message('developer', bodies['b'])
        other_record['type'] = 'function_call'
        self.assertEqual(self.q.count_developer_rule_bodies([other_block, other_record], bodies), {'a': 0, 'b': 0})
        for changed in ('RULE A', 'rule A\n', 'RULE C\n', 'RULE  A\n'):
            self.assertEqual(self.q.count_developer_rule_bodies(
                [message('developer', changed)], bodies), {'a': 0, 'b': 0})

    def test_rule_identity_requires_two_distinct_nonempty_bodies(self):
        for bodies in ({"a": "", "b": "B"}, {"a": "A", "b": "A"}, {"a": "A"}):
            with self.assertRaises(ValueError):
                self.q.count_developer_rule_bodies([], bodies)

    def observation(self):
        return {'pin': '0.164.0', 'positive_passed': True, 'session': 'session-a',
                'boundaries': [{'kind': k, 'session': 'session-a', 'completed': True,
                                'read_only': True, 'automatic': k != 'resume',
                                'body_counts': {'a': 1, 'b': 1}, 'effective_input_observed': True}
                               for k in ('resume', 'compact', 'compact')]}

    def test_each_boundary_requires_one_body_not_one_callback_or_marker(self):
        value = self.observation()
        self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'observed_single_copy')
        for i in range(3):
            for counts in ({'a': 2, 'b': 1}, {'a': 1, 'b': 2}, {'a': 0, 'b': 1},
                           {'a': 2, 'b': 0}, {'a': 0, 'b': 2},
                           {'a': True, 'b': 1}, {'a': 1}, {'x': 1, 'b': 1}):
                value = self.observation(); value['boundaries'][i]['body_counts'] = counts
                value['boundaries'][i]['callback_counts'] = {'a': 1, 'b': 1}
                with self.subTest(i=i, counts=counts):
                    self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'body_count_mismatch')

    def test_failed_positive_and_unobservable_input_never_pass(self):
        value = self.observation(); value['positive_passed'] = False
        self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')
        for i in range(3):
            for replacement in (False, None, 0, 1, 'yes'):
                value = self.observation(); value['boundaries'][i]['effective_input_observed'] = replacement
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'effective_input_unobserved')
            value = self.observation(); del value['boundaries'][i]['effective_input_observed']
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'effective_input_unobserved')

    def test_lifecycle_identity_completion_readonly_and_automatic_compactions(self):
        for field, changed in [('session', 'different'), ('completed', False), ('read_only', False)]:
            for i in range(3):
                value = self.observation(); value['boundaries'][i][field] = changed
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')
        for i in (1, 2):
            value = self.observation(); value['boundaries'][i]['automatic'] = False
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')

        for change in (lambda v: v['boundaries'].pop(), lambda v: v['boundaries'].reverse(), lambda v: v.update(pin='other')):
            value = self.observation(); change(value)
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')

    def test_missing_identity_and_nonboolean_lifecycle_evidence_fail(self):
        for changed in (None, '', 1, []):
            value = self.observation(); value['session'] = changed
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')
            for boundary in value['boundaries']:
                boundary['session'] = changed
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')
        for changed in (False, None, 0, 1, 'yes'):
            value = self.observation(); value['positive_passed'] = changed
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')
            for i in range(3):
                for key in ('completed', 'read_only', 'automatic'):
                    if key == 'automatic' and i == 0:
                        continue
                    value = self.observation(); value['boundaries'][i][key] = changed
                    self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')
        for i in range(3):
            for key in ('session', 'completed', 'read_only', 'automatic'):
                if key == 'automatic' and i == 0:
                    continue
                value = self.observation(); del value['boundaries'][i][key]
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')

    def test_markers_and_project_identities_are_fresh_between_fixtures(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixtures = [self.q.prepare_codex_rule_lifecycle(Path(tmp) / name) for name in ('one', 'two')]
            self.assertNotEqual(fixtures[0]['fallback'], fixtures[1]['fallback'])
            self.assertTrue(set(fixtures[0]['rule_bodies'].values()).isdisjoint(fixtures[1]['rule_bodies'].values()))
            self.assertNotEqual(fixtures[0]['market_name'], fixtures[1]['market_name'])
            commits = [subprocess.check_output(['git', '-C', str(f['project']), 'rev-parse', 'HEAD']) for f in fixtures]
            self.assertNotEqual(*commits)
