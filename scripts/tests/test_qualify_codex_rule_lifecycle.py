"""Offline Q9 fixtures and payload classification; no native qualification."""
import importlib.util
from pathlib import Path
import tempfile
import unittest


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
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name, ('AGENTS.md', 'AGENTS.override.md', 'CLAUDE.md'))
            for package, body in zip(f['packages'], f['rule_bodies'].values()):
                self.assertEqual((package / 'rule.txt').read_text(), body)
                self.assertTrue((package / 'hooks/hooks.json').is_file())
            self.assertFalse(f['qualified'])

    def test_existing_root_and_discovery_roots_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for path in (root, root / '.codex/fixture', root / '.agents/plugins/fixture'):
                with self.subTest(path=path), self.assertRaises(ValueError):
                    self.q.prepare_codex_rule_lifecycle(path)
            dangling = root / 'dangling'
            dangling.symlink_to(root / 'absent')
            with self.assertRaises(ValueError):
                self.q.prepare_codex_rule_lifecycle(dangling)

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
            for counts in ({'a': 2, 'b': 1}, {'a': 1, 'b': 2}, {'a': 0, 'b': 1}, {'a': True, 'b': 1}, {'a': 1}, {'x': 1, 'b': 1}):
                value = self.observation(); value['boundaries'][i]['body_counts'] = counts
                value['boundaries'][i]['callback_counts'] = {'a': 1, 'b': 1}
                with self.subTest(i=i, counts=counts):
                    self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'body_count_mismatch')

    def test_failed_positive_and_unobservable_input_never_pass(self):
        value = self.observation(); value['positive_passed'] = False
        self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')
        for i in range(3):
            value = self.observation(); value['boundaries'][i]['effective_input_observed'] = False
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
