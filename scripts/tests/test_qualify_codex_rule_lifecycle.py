"""Offline Q9 fixtures and payload classification; no native qualification."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import json
import os
import copy
import subprocess
import sys
from unittest import mock


SCRIPT = Path(__file__).parents[1] / 'qualify-plugin-rules.py'


class CodexLifecycleTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location('q9', SCRIPT)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)
        ambient = tempfile.TemporaryDirectory()
        self.addCleanup(ambient.cleanup)
        external = Path(ambient.name) / 'external'
        external.mkdir()
        env = self.q.fixture_environment(Path(ambient.name))
        subprocess.run(['git', 'init', '-q', str(external)], env=env, check=True)
        (external / 'sentinel.txt').write_text('Host repository sentinel.\n')
        subprocess.run(['git', '-C', str(external), 'add', 'sentinel.txt'], env=env, check=True)
        subprocess.run(['git', '-C', str(external), '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'test: initialize foreign repository'], env=env, check=True)
        patch = mock.patch.dict(os.environ, {'GIT_DIR': str(external / '.git'), 'GIT_WORK_TREE': str(external)})
        patch.start()
        self.addCleanup(patch.stop)

    def test_fresh_fixture_has_two_independent_rules_without_team_guidance(self):
        with tempfile.TemporaryDirectory() as tmp:
            previous_mask = os.umask(0o022)
            try:
                f = self.q.prepare_codex_rule_lifecycle(Path(tmp) / 'fixture')
            finally:
                os.umask(previous_mask)
            self.assertEqual(len(f['packages']), 2)
            self.assertEqual(f['root'].stat().st_mode & 0o777, 0o700)
            self.assertEqual(f['trust'], 'Ordinary user hook review required; never preapproved by fixture')
            self.assertFalse((f['root'] / 'home').exists())
            self.assertFalse((f['project'] / '.codex').exists())
            expected_files = {'delivery/.agents/plugins/marketplace.json'}
            for package in f['packages']:
                expected_files.update((package / leaf).relative_to(f['root']).as_posix() for leaf in ('rule.txt','hook.py','fixture.json','.codex-plugin/plugin.json','hooks/hooks.json'))
            actual_files = {path.relative_to(f['root']).as_posix() for path in f['root'].rglob('*') if path.is_file() and '.git' not in path.relative_to(f['root']).parts}
            self.assertEqual(actual_files, expected_files)
            self.assertEqual(len(set(f['rule_bodies'].values())), 2)
            self.assertEqual(f['pin'], '0.164.0')
            market = json.loads((f['marketplace'] / '.agents/plugins/marketplace.json').read_text())
            self.assertEqual(market, {'name': f['market_name'], 'plugins': [
                {'name': p.name, 'source': {'source': 'local', 'path': './packages/' + p.name}}
                for p in f['packages']]})
            parents = subprocess.check_output(['git', '-C', str(f['project']), 'show', '-s', '--format=%P', 'HEAD'], text=True, env=self.q.fixture_environment(f['root']))
            self.assertEqual(parents.strip(), '')
            self.assertEqual(subprocess.check_output(['git', '-C', str(f['project']), 'ls-tree', '-r', '--name-only', 'HEAD'], env=self.q.fixture_environment(f['root'])), b'')
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
            missing_home = root / 'missing-home'
            with mock.patch.object(Path, 'home', return_value=missing_home), self.assertRaises(ValueError):
                self.q.prepare_codex_rule_lifecycle(missing_home)
            self.assertFalse(missing_home.exists())
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
        for bodies in ({"a": "", "b": "B"}, {"a": "A", "b": "A"}, {"a": "A"}, {"x": "A", "b": "B"}, {"a": 1, "b": "B"}, {"a": "X\n", "b": "XX\n"}):
            with self.assertRaises(ValueError):
                self.q.count_developer_rule_bodies([], bodies)

    def observation(self):
        return {'pin': '0.164.0', 'positive_passed': True, 'session': 'session-a',
                'positive': {'session': 'session-a', 'completed': True, 'read_only': True, 'body_counts': {'a': 1, 'b': 1}},
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
                           {'a': True, 'b': 1}, {'a': 1}, {'x': 1, 'b': 1}, None, [], 'invalid'):
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
            commits = [subprocess.check_output(['git', '-C', str(f['project']), 'rev-parse', 'HEAD'], env=self.q.fixture_environment(f['root'])) for f in fixtures]
            self.assertNotEqual(*commits)
            messages = [subprocess.check_output(['git', '-C', str(f['project']), 'show', '-s', '--format=%B', 'HEAD'], env=self.q.fixture_environment(f['root'])) for f in fixtures]
            self.assertNotEqual(*messages)
            self.assertTrue(all(message.startswith(b'test: initialize rule lifecycle fixture ') for message in messages))

    def test_structured_positive_requires_startup_counts_and_matching_session(self):
        for key, changed in [('session','other'), ('completed',False), ('read_only',False), ('body_counts',{'a':2,'b':1}), ('body_counts',{'a':1,'b':0}), ('body_counts',{'a':True,'b':1}), ('body_counts',None)]:
            value = self.observation()
            value['positive'][key] = changed
            with self.subTest(key=key, changed=changed):
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')
        value = self.observation()
        del value['positive']
        self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')

    def test_positive_failure_precedes_lifecycle_and_count_failures(self):
        for boundary in (0,1,2):
            value = self.observation()
            value['positive_passed'] = False
            value['boundaries'][boundary]['body_counts'] = {'a':2,'b':1}
            value['boundaries'][boundary]['completed'] = False
            self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'positive_control_failed')

    def test_all_lifecycle_controls_precede_any_count_failure(self):
        for count_at, invalid_at in ((0,2),(2,1)):
            for key, changed in [('session','other'),('completed',False),('read_only',False),('automatic',False)]:
                value = self.observation()
                value['boundaries'][count_at]['body_counts'] = {'a':2,'b':1}
                value['boundaries'][invalid_at][key] = changed
                with self.subTest(count_at=count_at, invalid_at=invalid_at, key=key):
                    self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')

    def test_count_failure_precedes_unobserved_effective_input(self):
        for count_at in (0,1,2):
            for unknown_at in (0,1,2):
                value = self.observation()
                value['boundaries'][count_at]['body_counts'] = {'a':2,'b':1}
                value['boundaries'][unknown_at]['effective_input_observed'] = False
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'body_count_mismatch')

    def test_every_boundary_kind_is_required_in_order(self):
        for kinds in [('resume','resume','resume'),('resume','resume','compact'),('resume','compact','resume'),('compact','resume','compact')]:
            value = self.observation()
            for row, kind in zip(value['boundaries'], kinds):
                row['kind'] = kind
            with self.subTest(kinds=kinds):
                self.assertEqual(self.q.classify_codex_rule_lifecycle(value), 'lifecycle_control_failed')

    def test_every_text_block_and_occurrence_is_counted(self):
        bodies = {'a':'RULE A\n','b':'RULE B\n'}
        message = {'type':'message','role':'developer','content':[{'type':'input_image'}, {'type':'input_text','text':bodies['a']}, {'type':'input_text','text':bodies['a']*2+bodies['b']}]}
        self.assertEqual(self.q.count_developer_rule_bodies([message],bodies), {'a':3,'b':1})
        for content in (None, 'invalid content'):
            message['content'] = content
            with self.assertRaises(ValueError):
                self.q.count_developer_rule_bodies([message],bodies)

    def test_retained_packet_replays_bounded_counts_and_incomplete_lifecycle(self):
        packet = SCRIPT.resolve().parents[1] / 'docs/technotes/evidence/2026-10-24-codex-rule-lifecycle'
        result = self.q.replay_codex_rule_lifecycle(packet)
        self.assertEqual(result, {'classification':'lifecycle_control_failed', 'recorded_body_counts':[{'a':1,'b':1},{'a':2,'b':2},{'a':1,'b':1}]})
        self.assertEqual(self.q.classify_codex_rule_lifecycle(json.loads((packet/'assessment.json').read_text())), 'lifecycle_control_failed')
        process = subprocess.run([sys.executable,str(SCRIPT),'--replay-codex-rule-lifecycle',str(packet)],capture_output=True,text=True,check=True)
        self.assertEqual(json.loads(process.stdout), result)
        original = json.loads((packet/'assessment.json').read_text())
        for mutation in ('count','status','positive_session','positive_count'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                target = Path(temporary)
                for path in packet.iterdir():
                    if path.is_file():
                        (target/path.name).write_bytes(path.read_bytes())
                changed = copy.deepcopy(original)
                if mutation == 'count':
                    changed['boundaries'][1]['body_counts'] = {'a':1,'b':1}
                elif mutation == 'status':
                    changed['status'] = 'PASS'
                elif mutation == 'positive_session':
                    changed['positive']['session'] = 'other'
                else:
                    changed['positive']['body_counts'] = {'a':2,'b':2}
                (target/'assessment.json').write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    self.q.replay_codex_rule_lifecycle(target)

    def test_configured_codex_roots_are_rejected_before_fixture_effects(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            configured = base / 'ConfiguredCodex'
            root = configured / 'fixture'
            with mock.patch.dict(os.environ, {'CODEX_HOME': str(configured)}), self.assertRaises(ValueError):
                self.q.prepare_codex_rule_lifecycle(root)
            self.assertFalse(root.exists())
