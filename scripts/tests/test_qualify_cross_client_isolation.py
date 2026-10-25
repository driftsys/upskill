import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)

PINS = {'claude': '2.1.308', 'copilot': '1.0.103', 'codex': '0.164.0', 'opencode': '2.1.4'}


class CrossClientIsolation(unittest.TestCase):
    def fixture(self, root):
        return q.prepare_cross_client_isolation(root, pins=PINS)

    def observation(self, row):
        def phase(project):
            return dict(pin=row['pin'], platform='macOS arm64', cwd=str(project),
                        scope='session', state='success', complete=True,
                        catalog_complete=True, catalog_present=False,
                        invocation_complete=True, namespace=row['namespace'],
                        response=row['absent_marker'], activations=0,
                        unexpected_tools=False, invocation_attempted=True, invocation_outcome='not_found', content_activations=0)
        positive = phase(row['positive_project'])
        positive.update(catalog_present=True, response=row['marker'], activations=1,
                        source_path=str(row['native_package']), invocation_outcome='loaded')
        return dict(positive=positive, baseline=phase(row['candidate_project']),
                    candidate=phase(row['candidate_project']),
                    selector=dict(available=True, status='selected', completed=True),
                    trees_after=row['trees'], packages_after=row['packages'],
                    preservation={'before': {'config': {'present': False, 'sha256': None, 'mode': None}},
                                  'after': {'config': {'present': False, 'sha256': None, 'mode': None}}})

    def test_same_identity_and_complete_skill_tree_across_envelopes(self):
        with tempfile.TemporaryDirectory() as temporary:
            f = self.fixture(Path(temporary) / 'fresh')
            namespaces = []
            for target, row in f['targets'].items():
                self.assertEqual(row['client'], target)
                self.assertEqual(row['pin'], PINS[target])
                self.assertEqual(row['scope'], 'session')
                self.assertEqual(row['native_intended'], target)
                self.assertNotEqual(row['foreign_intended'], target)
                self.assertTrue(row['native_manifest'].is_file())
                self.assertTrue(row['foreign_manifest'].is_file())
                namespaces.append(row['namespace'])
                trees = [q.cross_client_skill_tree(row[key]) for key in ('native_package', 'foreign_package')]
                self.assertEqual(trees[0], trees[1])
                self.assertEqual(row['trees'], {'native': trees[0], 'foreign': trees[1]})
                self.assertEqual(len(trees[0]), 2)
                for key in ('native_package', 'foreign_package'):
                    self.assertFalse(row[key].is_relative_to(row['positive_project']))
                    self.assertFalse(row[key].is_relative_to(row['candidate_project']))
                claude_package = row['native_package'] if target == 'claude' else row['foreign_package']
                copilot_package = row['foreign_package'] if target == 'claude' else row['native_package']
                self.assertEqual(json.loads((claude_package / '.claude-plugin/plugin.json').read_text())['name'], row['plugin_name'])
                self.assertEqual(json.loads((copilot_package / 'plugin.json').read_text())['name'], row['plugin_name'])
            self.assertEqual(len(set(namespaces)), 2)
            paths = list(f['root'].rglob('*'))
            self.assertFalse(any(p.is_symlink() or p.name in ('CLAUDE.md', 'AGENTS.md') for p in paths))

    def test_refuses_existing_discovery_and_dangling_roots_without_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for rel in ('.claude/q11', '.github/q11', '.agents/skills/q11', '.copilot/q11'):
                root = base / rel
                with self.assertRaises(ValueError):
                    self.fixture(root)
                self.assertFalse(root.exists())
            link = base / 'dangling'
            link.symlink_to(base / 'missing')
            with self.assertRaises(ValueError):
                self.fixture(link)
            self.assertTrue(link.is_symlink())

    def test_tree_fingerprint_includes_resources_modes_and_rejects_symlinks(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            package = row['foreign_package']
            resource = next(package.glob('skills/*/resource.txt'))
            before = q.cross_client_skill_tree(package)
            resource.write_text('changed')
            self.assertNotEqual(q.cross_client_skill_tree(package), before)
            before = q.cross_client_skill_tree(package)
            resource.chmod(0o600 if (resource.stat().st_mode & 0o777) != 0o600 else 0o640)
            self.assertNotEqual(q.cross_client_skill_tree(package), before)
            resource.unlink()
            resource.symlink_to(package / 'missing')
            with self.assertRaises(ValueError):
                q.cross_client_skill_tree(package)

    def test_all_targets_accept_only_complete_paired_negative(self):
        with tempfile.TemporaryDirectory() as temporary:
            for row in self.fixture(Path(temporary) / 'fresh')['targets'].values():
                result = q.classify_cross_client_isolation(self.observation(row), row)
                self.assertEqual(result, {'isolation': 'isolation_observed', 'preservation': 'unchanged'})

    def test_failed_positive_and_contaminated_baseline_block_negative(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            for phase, field, value in [('positive', 'state', 'error'), ('positive', 'complete', False),
                                        ('positive', 'response', 'wrong'), ('positive', 'catalog_present', False),
                                        ('positive', 'source_path', '/unrelated'), ('baseline', 'catalog_present', True),
                                        ('baseline', 'activations', 1), ('baseline', 'invocation_complete', False)]:
                with self.subTest(phase=phase, field=field):
                    o = self.observation(row)
                    o[phase][field] = value
                    self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'],
                                     'blocked_positive' if phase == 'positive' else ('baseline_contaminated' if field in ('catalog_present', 'activations') else 'blocked_baseline'))

    def test_activation_wins_over_missing_catalog_and_preservation_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['copilot']
            o = self.observation(row)
            o['candidate'].update(activations=1, catalog_complete=False, state='error')
            o['preservation']['after'] = {'config': {'present': True, 'mode': 0o600, 'sha256': 'b' * 64}}
            result = q.classify_cross_client_isolation(o, row)
            self.assertEqual(result, {'isolation': 'activation_observed', 'preservation': 'preservation_failed'})

    def test_catalog_exposure_never_passes(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['copilot']
            o = self.observation(row)
            o['candidate']['catalog_present'] = True
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'catalog_exposure')

    def test_incomplete_or_malformed_candidate_never_passes(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            for field, value in [('catalog_complete', False), ('invocation_complete', False),
                                 ('complete', 1), ('state', 'error'), ('invocation_attempted', False), ('invocation_outcome', 'not_attempted'), ('response', row['absent_marker'] + '\n'),
                                 ('activations', False), ('unexpected_tools', True), ('catalog_present', None)]:
                with self.subTest(field=field):
                    o = self.observation(row)
                    o['candidate'][field] = value
                    self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'incomplete_evidence')

    def test_metadata_drift_and_resource_drift_invalidate_control(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            for field in ('pin', 'platform', 'cwd', 'scope', 'namespace'):
                o = self.observation(row)
                o['candidate'][field] = 'different'
                self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'control_metadata_failed')
            o = copy.deepcopy(self.observation(row))
            o['trees_after']['foreign'].pop(next(iter(o['trees_after']['foreign'])))
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'invalid_pair')

    def test_selector_unavailable_is_distinct_from_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            o = self.observation(row)
            o['selector'] = {'available': False, 'status': 'unavailable', 'completed': True}
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'selector_unavailable')
            o['selector'] = {'available': True, 'status': 'rejected', 'completed': True}
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'isolation_observed')

    def test_preservation_computed_independently_and_requires_valid_receipts(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            for evidence in ({}, {'before': {}, 'after': {}},
                             {'before': {'config': 'invented'}, 'after': {'config': 'invented'}}):
                o = self.observation(row)
                o['preservation'] = evidence
                self.assertEqual(q.classify_cross_client_isolation(o, row)['preservation'], 'preservation_failed')
            o = self.observation(row)
            receipt = {'present': True, 'sha256': 'a' * 64, 'mode': 0o600}
            o['preservation'] = {'before': {'config': receipt}, 'after': {'config': dict(receipt, mode=0o644)}}
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'isolation_observed')
            self.assertEqual(q.classify_cross_client_isolation(o, row)['preservation'], 'preservation_failed')


    def test_content_leakage_and_selector_conversion_are_activation(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['copilot']
            for catalog in (True, False):
                o = self.observation(row)
                o['candidate'].update(response='\n\n' + row['marker'], catalog_present=catalog)
                self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'activation_observed')
            for status in ('installed', 'converted'):
                o = self.observation(row)
                o['selector']['status'] = status
                self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'activation_observed')
            o = self.observation(row)
            o['candidate']['content_activations'] = 1
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'activation_observed')

    def test_manifest_mutation_invalidates_pair(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            o = self.observation(row)
            manifest = row['foreign_manifest']
            manifest.write_text('{}\n')
            o['packages_after'] = dict(row['packages'], foreign=q.cross_client_package_tree(row['foreign_package']))
            self.assertEqual(q.classify_cross_client_isolation(o, row)['isolation'], 'invalid_pair')


class CrossClientNativeReplay(unittest.TestCase):
    def row(self, client):
        return {'client': client, 'pin': PINS[client], 'scope': 'session',
                'positive_project': '/fixture/positive', 'candidate_project': '/fixture/candidate',
                'native_package': '/fixture/native/package', 'foreign_package': '/fixture/foreign/package',
                'plugin_name': 'package', 'skill_name': 'probe', 'namespace': 'package:probe',
                'marker': 'PRESENT', 'absent_marker': 'ABSENT'}

    def copilot_events(self):
        return [
            {'type': 'session.skills_loaded', 'data': {'skills': [{'commandName': 'package:probe', 'path': '/fixture/native/package/skills/probe/SKILL.md', 'enabled': True}]}},
            {'type': 'tool.execution_start', 'data': {'toolName': 'skill', 'toolCallId': 'tool1', 'arguments': {'skill': 'package:probe'}}},
            {'type': 'tool.execution_complete', 'data': {'toolCallId': 'tool1', 'success': True}},
            {'type': 'assistant.message', 'data': {'content': 'PRESENT'}},
            {'type': 'result', 'exitCode': 0},
        ]

    def test_copilot_requires_exact_correlated_namespaced_success(self):
        row = self.row('copilot')
        events = self.copilot_events()
        result = q.inspect_cross_client_positive(events, row, exit_code=0)
        self.assertEqual(result['activations'], 1)
        self.assertTrue(result['catalog_present'])
        self.assertEqual(result['source_path'], row['native_package'])
        self.assertFalse(result['unexpected_tools'])
        events[2]['data']['success'] = False
        events.insert(3, {'type': 'tool.execution_start', 'data': {'toolName': 'skill', 'toolCallId': 'tool2', 'arguments': {'skill': 'probe'}}})
        events.insert(4, {'type': 'tool.execution_complete', 'data': {'toolCallId': 'tool2', 'success': True}})
        result = q.inspect_cross_client_positive(events, row, exit_code=0)
        self.assertEqual(result['activations'], 0)
        self.assertTrue(result['unexpected_tools'])

    def test_native_errors_or_truncated_streams_cannot_complete(self):
        for mutations in ('exit', 'missing_result', 'malformed', 'timeout', 'unsuccessful_result'):
            events = self.copilot_events()
            kwargs = dict(exit_code=0)
            if mutations == 'exit': kwargs['exit_code'] = 1
            if mutations == 'missing_result': events.pop()
            if mutations == 'malformed': kwargs['malformed_stream'] = True
            if mutations == 'timeout': kwargs['timed_out'] = True
            if mutations == 'unsuccessful_result': events[-1]['exitCode'] = 1
            result = q.inspect_cross_client_positive(events, self.row('copilot'), **kwargs)
            self.assertEqual(result['state'], 'error')
            self.assertFalse(result['invocation_complete'])

    def test_claude_error_subtype_success_is_still_failed(self):
        row = self.row('claude')
        events = [{'type': 'system', 'subtype': 'init', 'slash_commands': ['package:probe'],
                   'plugins': [{'name': 'package', 'path': row['native_package']}]},
                  {'type': 'result', 'subtype': 'success', 'is_error': True,
                   'permission_denials': [], 'result': 'private provider error'}]
        result = q.inspect_cross_client_positive(events, row, exit_code=1)
        self.assertEqual(result['state'], 'error')
        self.assertTrue(result['catalog_present'])
        self.assertEqual(result['activations'], 0)
        self.assertIsNone(result['response'])

    def test_wrong_native_source_path_cannot_attribute_positive(self):
        events = self.copilot_events()
        events[0]['data']['skills'][0]['path'] = '/other/skills/probe/SKILL.md'
        result = q.inspect_cross_client_positive(events, self.row('copilot'), exit_code=0)
        self.assertIsNone(result['source_path'])

    def test_missing_native_tool_identity_cannot_count_as_activation(self):
        events = self.copilot_events()
        del events[1]['data']['toolCallId']
        del events[2]['data']['toolCallId']
        result = q.inspect_cross_client_positive(events, self.row('copilot'), exit_code=0)
        self.assertEqual(result['activations'], 0)
        self.assertTrue(result['unexpected_tools'])

    def test_retained_native_positives_replay_as_blocked(self):
        packet = Path(__file__).parents[2] / 'docs/technotes/evidence/2026-10-25-cross-client-package-isolation'
        fixture = json.loads((packet / 'fixture.json').read_text())
        for client, row in fixture['targets'].items():
            events = json.loads((packet / (client + '-positive-native-selection.json')).read_text())
            receipt = json.loads((packet / (client + '-positive-receipt.json')).read_text())
            observation = json.loads((packet / (client + '-observation.json')).read_text())
            positive = q.inspect_cross_client_positive(events, row, exit_code=receipt['exit_code'], timed_out=receipt['timed_out'])
            positive['cwd'] = receipt['cwd']
            self.assertEqual(positive, observation['positive'])
            result = q.classify_cross_client_isolation(observation, row)
            self.assertEqual(result, {'isolation': 'blocked_positive', 'preservation': 'unchanged'})
            self.assertEqual(result, json.loads((packet / (client + '-classification.json')).read_text()))


    def test_no_tool_call_is_not_an_explicit_invocation(self):
        events = [e for e in self.copilot_events() if not e['type'].startswith('tool.')]
        result = q.inspect_cross_client_positive(events, self.row('copilot'), exit_code=0)
        self.assertFalse(result['invocation_complete'])
        self.assertFalse(result['invocation_attempted'])
        self.assertEqual(result['invocation_outcome'], 'not_attempted')
