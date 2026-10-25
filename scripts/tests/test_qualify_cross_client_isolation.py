import copy
import datetime
import importlib.util
import hashlib
import json
import os
import subprocess
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
                self.assertNotEqual(row['positive_project'], row['candidate_project'])
                roots = []
                for key in ('positive_project', 'candidate_project'):
                    project = row[key]
                    self.assertTrue((project / '.git').is_dir())
                    result = subprocess.run(['git', '-C', str(project), 'rev-list', '--parents', '-n', '1', 'HEAD'],
                                            env=q.fixture_environment(f['root']), capture_output=True, text=True, check=True)
                    commit = result.stdout.split()
                    self.assertEqual(len(commit), 1)
                    roots.append(commit[0])
                self.assertNotEqual(roots[0], roots[1])
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
            for phase in ('positive', 'baseline', 'candidate'):
                for field in ('pin', 'platform', 'cwd', 'scope', 'namespace'):
                    with self.subTest(phase=phase, field=field):
                        o = self.observation(row)
                        o[phase][field] = 'different'
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


    def test_initially_unequal_pair_is_invalid_even_without_later_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            key = next(iter(row['trees']['foreign']))
            row['trees']['foreign'][key]['sha256'] = 'f' * 64
            observation = self.observation(row)
            self.assertEqual(observation['trees_after'], row['trees'])
            self.assertEqual(q.classify_cross_client_isolation(observation, row)['isolation'], 'invalid_pair')

    def test_equal_present_receipts_require_valid_digest_and_mode(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = self.fixture(Path(temporary) / 'fresh')['targets']['claude']
            valid = {'present': True, 'sha256': 'a' * 64, 'mode': 0o600}
            observation = self.observation(row)
            observation['preservation'] = {'before': {'config': valid}, 'after': {'config': dict(valid)}}
            self.assertEqual(q.classify_cross_client_isolation(observation, row)['preservation'], 'unchanged')
            for field, value in (('sha256', None), ('sha256', 'a' * 63), ('sha256', 'z' * 64),
                                 ('mode', None), ('mode', True), ('mode', -1), ('mode', 0o10000)):
                with self.subTest(field=field, value=value):
                    receipt = dict(valid, **{field: value})
                    observation['preservation'] = {'before': {'config': receipt}, 'after': {'config': dict(receipt)}}
                    self.assertEqual(q.classify_cross_client_isolation(observation, row)['preservation'], 'preservation_failed')
            for missing in ('sha256', 'mode'):
                receipt = {key: value for key, value in valid.items() if key != missing}
                observation['preservation'] = {'before': {'config': receipt}, 'after': {'config': dict(receipt)}}
                self.assertEqual(q.classify_cross_client_isolation(observation, row)['preservation'], 'preservation_failed')

    def test_missing_or_invalid_campaign_pins_are_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'fresh'
            for client in PINS:
                for value in (None, '', 123):
                    with self.subTest(client=client, value=value):
                        with self.assertRaises(ValueError):
                            q.prepare_cross_client_isolation(root, pins=dict(PINS, **{client: value}))
                        self.assertFalse(root.exists())
                with self.assertRaises(ValueError):
                    q.prepare_cross_client_isolation(root, pins={key: value for key, value in PINS.items() if key != client})
                self.assertFalse(root.exists())

    def test_complete_package_rejects_links_and_nonregular_entries(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            row = self.fixture(base / 'fresh')['targets']['claude']
            alias = base / 'alias'
            alias.symlink_to(row['native_package'], target_is_directory=True)
            with self.assertRaises(ValueError):
                q.cross_client_package_tree(alias)
            regular_target = base / 'regular-target.txt'
            regular_target.write_text('Existing external file.\n')
            directory_target = base / 'directory-target'
            directory_target.mkdir()
            (directory_target / 'nested.txt').write_text('Existing external directory.\n')
            entry = row['native_package'] / 'external-link'
            for target in (regular_target, directory_target):
                with self.subTest(target=target.name):
                    entry.symlink_to(target, target_is_directory=target.is_dir())
                    try:
                        self.assertTrue(entry.exists())
                        with self.assertRaises(ValueError):
                            q.cross_client_package_tree(row['native_package'])
                    finally:
                        entry.unlink()
            manifest = row['native_manifest']
            manifest.unlink()
            manifest.symlink_to(base / 'missing')
            with self.assertRaises(ValueError):
                q.cross_client_package_tree(row['native_package'])
            manifest.unlink()
            manifest.write_text('{}\n')
            if hasattr(os, 'mkfifo'):
                os.mkfifo(row['native_package'] / 'pipe')
                with self.assertRaises(ValueError):
                    q.cross_client_package_tree(row['native_package'])


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
        events = self.claude_events()
        events[-1].update(is_error=True, result='private provider error')
        result = q.inspect_cross_client_positive(events, row, exit_code=0)
        self.assertEqual(result['state'], 'error')
        self.assertTrue(result['catalog_present'])
        self.assertEqual(result['activations'], 1)
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


    def claude_events(self):
        return [
            {'type': 'system', 'subtype': 'init', 'slash_commands': ['package:probe'],
             'plugins': [{'name': 'package', 'path': '/fixture/native/package'}]},
            {'type': 'assistant', 'message': {'content': [{'type': 'tool_use', 'id': 'tool1',
                                                         'name': 'Skill', 'input': {'skill': 'package:probe'}}]}},
            {'type': 'user', 'message': {'content': [{'type': 'tool_result', 'tool_use_id': 'tool1',
                                                    'is_error': False, 'content': 'fixture body'}]}},
            {'type': 'result', 'subtype': 'success', 'is_error': False, 'permission_denials': [], 'result': 'PRESENT'},
        ]

    def test_claude_success_requires_correlated_successful_tool_result(self):
        row = self.row('claude')
        result = q.inspect_cross_client_positive(self.claude_events(), row, exit_code=0)
        self.assertEqual(result['state'], 'success')
        self.assertEqual(result['response'], row['marker'])
        self.assertEqual(result['activations'], 1)
        self.assertEqual(result['invocation_outcome'], 'loaded')
        self.assertTrue(result['invocation_complete'])
        for field, value in (('tool_use_id', 'unrelated'), ('tool_use_id', None), ('is_error', True)):
            with self.subTest(field=field, value=value):
                events = self.claude_events()
                events[2]['message']['content'][0][field] = value
                result = q.inspect_cross_client_positive(events, row, exit_code=0)
                self.assertEqual(result['activations'], 0)
                self.assertFalse(result['invocation_complete'])

    def test_retained_slash_retry_keeps_raw_response_and_failed_verdict(self):
        packet = Path(__file__).parents[2] / 'docs/technotes/evidence/2026-10-25-cross-client-package-isolation/copilot-slash-retry'
        fixture = json.loads((packet / 'fixture.json').read_text())
        events = json.loads((packet / 'native-selection.json').read_text())
        summary = json.loads((packet / 'summary.json').read_text())
        receipt = json.loads((packet / 'positive-receipt.json').read_text())
        self.assertEqual(receipt['command'][-1], '/' + fixture['namespace'])
        self.assertEqual(receipt['exit_code'], 0)
        self.assertFalse(receipt['timed_out'])
        self.assertEqual(next(e['exitCode'] for e in events if e['type'] == 'result'), 0)
        response = next(e['data']['content'] for e in reversed(events) if e['type'] == 'assistant.message')
        self.assertEqual(response, '\n\n' + fixture['marker'])
        self.assertNotEqual(response, fixture['marker'])
        self.assertEqual(summary['response_exact'], response == fixture['marker'])
        self.assertEqual(summary['positive'], 'failed_exact_response')
        self.assertEqual(summary['baseline'], 'not_run')
        self.assertEqual(summary['foreign_candidate'], 'not_run')
        starts = [e['data'] for e in events if e['type'] == 'tool.execution_start']
        self.assertEqual(len(starts), 1)
        self.assertEqual(starts[0]['toolName'], 'skill')
        self.assertEqual(starts[0]['arguments']['skill'], fixture['skill_name'])
        ends = [e['data'] for e in events if e['type'] == 'tool.execution_complete'
                and e['data'].get('toolCallId') == starts[0]['toolCallId']]
        self.assertEqual(len(ends), 1)
        self.assertIs(ends[0]['success'], True)
        self.assertIs(summary['bare_skill_load_succeeded'], True)

    def test_retained_packet_hashes_cover_all_public_evidence(self):
        packet = Path(__file__).parents[2] / 'docs/technotes/evidence/2026-10-25-cross-client-package-isolation'
        manifest = json.loads((packet / 'receipt-hashes.json').read_text())
        paths = {str(path.relative_to(packet)): path for path in packet.rglob('*')
                 if path.is_file() and path.name != 'receipt-hashes.json'}
        self.assertEqual(set(manifest), set(paths))
        for name, path in paths.items():
            with self.subTest(file=name):
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), manifest[name])


class ClaudeInlineBookkeeping(unittest.TestCase):
    def states(self):
        before = {'oauthAccount': {'id': 'manual-account'}, 'skills': ['manual-source'],
                  'cachedGrowthBookFeaturesAt': 10,
                  'pluginUsage': {'manual@market': {'usageCount': 2}},
                  'skillUsage': {'manual:skill': {'usageCount': 3}}}
        after = copy.deepcopy(before)
        after['cachedGrowthBookFeaturesAt'] = 20
        after['pluginUsage']['package@inline'] = {'usageCount': 1, 'lastUsedAt': 20, 'lastUsedNumStartups': 1}
        after['skillUsage']['package:probe'] = {'usageCount': 1, 'lastUsedAt': 20}
        return before, after

    def classify(self, before, after, before_mode=0o600, after_mode=0o600):
        return q.classify_claude_inline_bookkeeping(json.dumps(before).encode(), json.dumps(after).encode(),
                                                    before_mode, after_mode, plugin_name='package', namespace='package:probe')

    def test_only_owned_inline_usage_and_cache_timestamp_are_restorable(self):
        before, after = self.states()
        self.assertEqual(self.classify(before, before), 'unchanged')
        self.assertEqual(self.classify(before, after), 'native_bookkeeping_residue')

    def test_manual_auth_discovery_or_unrelated_usage_changes_are_rejected(self):
        before, after = self.states()
        for field, value in [('oauthAccount', {'id': 'changed-account'}), ('skills', ['changed-source']),
                             ('pluginUsage', {'manual@market': {'usageCount': 99}}),
                             ('skillUsage', {'manual:skill': {'usageCount': 99}})]:
            with self.subTest(field=field):
                changed = copy.deepcopy(after)
                changed[field] = value
                self.assertEqual(self.classify(before, changed), 'preservation_failed')
        self.assertEqual(self.classify(before, after, after_mode=0o644), 'preservation_failed')

    def test_unexpected_usage_fields_and_scalar_types_are_rejected(self):
        before, after = self.states()
        for field, value in [('cachedGrowthBookFeaturesAt', True), ('cachedGrowthBookFeaturesAt', 1)]:
            changed = copy.deepcopy(after)
            changed[field] = value
            original = copy.deepcopy(before)
            if value is True:
                original[field] = 0
            self.assertEqual(self.classify(original, changed), 'preservation_failed')
        original = copy.deepcopy(before)
        original['cachedGrowthBookFeaturesAt'] = True
        self.assertEqual(self.classify(original, after), 'preservation_failed')
        for scope, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            changed = copy.deepcopy(after)
            changed[scope][key]['extra'] = 1
            self.assertEqual(self.classify(before, changed), 'preservation_failed')
            changed = copy.deepcopy(after)
            changed[scope][key]['usageCount'] = True
            self.assertEqual(self.classify(before, changed), 'preservation_failed')


    def test_mode_receipts_reject_noninteger_and_out_of_range_values(self):
        before, after = self.states()
        for left, right in [(0o600, float(0o600)), (float(0o600), 0o600),
                            (True, 1), (1, True), (False, 0), (0, False),
                            (-1, -1), (0o10000, 0o10000)]:
            with self.subTest(before_mode=left, after_mode=right):
                self.assertEqual(self.classify(before, after, before_mode=left, after_mode=right), 'preservation_failed')


    def test_usage_numeric_boundaries_reject_zero_counts_and_negative_values(self):
        before, after = self.states()
        for field, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            boundary = copy.deepcopy(after)
            boundary[field][key] = {name: 1 if name == 'usageCount' else 0 for name in boundary[field][key]}
            self.assertEqual(self.classify(before, boundary), 'native_bookkeeping_residue')
            for name in boundary[field][key]:
                for invalid in ([0, -1] if name == 'usageCount' else [-1]):
                    with self.subTest(container=field, value=name, invalid=invalid):
                        changed = copy.deepcopy(boundary)
                        changed[field][key][name] = invalid
                        self.assertEqual(self.classify(before, changed), 'preservation_failed')

    def test_timestamp_only_and_independent_usage_additions_are_restorable(self):
        before, populated = self.states()
        timestamp_only = copy.deepcopy(before)
        timestamp_only['cachedGrowthBookFeaturesAt'] = populated['cachedGrowthBookFeaturesAt']
        self.assertEqual(self.classify(before, timestamp_only), 'native_bookkeeping_residue')
        for field, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            independent = copy.deepcopy(before)
            independent[field][key] = populated[field][key]
            self.assertEqual(self.classify(before, independent), 'native_bookkeeping_residue')
        empty = {key: value for key, value in before.items() if key not in ('pluginUsage', 'skillUsage')}
        self.assertEqual(self.classify(empty, empty), 'unchanged')
        for field, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            added = copy.deepcopy(empty)
            added[field] = {key: populated[field][key]}
            self.assertEqual(self.classify(empty, added), 'native_bookkeeping_residue')
        timestamp_only = dict(empty, cachedGrowthBookFeaturesAt=20)
        self.assertEqual(self.classify(empty, timestamp_only), 'native_bookkeeping_residue')

    def test_inconsistent_fixture_names_are_rejected_even_for_unchanged_bytes(self):
        before, _ = self.states()
        data = json.dumps(before).encode()
        for plugin, namespace in [('', 'package:probe'), ('package', 'other:probe'), ('package', 'probe')]:
            with self.subTest(plugin=plugin, namespace=namespace):
                self.assertEqual(q.classify_claude_inline_bookkeeping(data, data, 0o600, 0o600,
                    plugin_name=plugin, namespace=namespace), 'preservation_failed')

    def test_malformed_and_nonobject_configurations_are_rejected(self):
        before, after = self.states()
        valid_before, valid_after = json.dumps(before).encode(), json.dumps(after).encode()
        for invalid in [b'{', b'[]', b'null', b'17', b'true', b'"text"', None]:
            for side in ('before', 'after'):
                with self.subTest(side=side, invalid=invalid):
                    left, right = (invalid, valid_after) if side == 'before' else (valid_before, invalid)
                    self.assertEqual(q.classify_claude_inline_bookkeeping(left, right, 0o600, 0o600,
                        plugin_name='package', namespace='package:probe'), 'preservation_failed')

    def test_nonmapping_usage_containers_and_owned_entries_are_rejected(self):
        before, after = self.states()
        for field, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            for invalid in (None, [], 1, 'not-a-map', True):
                for side in ('before', 'after'):
                    with self.subTest(container=field, side=side, invalid=invalid):
                        left, right = copy.deepcopy(before), copy.deepcopy(after)
                        (left if side == 'before' else right)[field] = invalid
                        self.assertEqual(self.classify(left, right), 'preservation_failed')
                with self.subTest(entry=field, invalid=invalid):
                    changed = copy.deepcopy(after)
                    changed[field][key] = invalid
                    self.assertEqual(self.classify(before, changed), 'preservation_failed')

    def test_existing_owned_keys_removed_containers_and_format_only_changes_are_rejected(self):
        before, after = self.states()
        for field, key in [('pluginUsage', 'package@inline'), ('skillUsage', 'package:probe')]:
            previous = copy.deepcopy(before)
            previous[field][key] = after[field][key]
            changed = dict(previous, cachedGrowthBookFeaturesAt=20)
            self.assertEqual(self.classify(previous, changed), 'preservation_failed')
            previous[field] = {}
            changed = copy.deepcopy(previous)
            changed.pop(field)
            changed['cachedGrowthBookFeaturesAt'] = 20
            self.assertEqual(self.classify(previous, changed), 'preservation_failed')
        self.assertEqual(q.classify_claude_inline_bookkeeping(
            json.dumps(before).encode(), json.dumps(before, indent=2).encode(), 0o600, 0o600,
            plugin_name='package', namespace='package:probe'), 'preservation_failed')


class ClaudeAuthenticatedRetry(unittest.TestCase):
    packet = Path(__file__).parents[2] / 'docs/technotes/evidence/2026-10-25-cross-client-package-isolation/claude-authenticated-retry'

    def read(self, name):
        return json.loads((self.packet / name).read_text())

    def test_positive_baseline_and_foreign_activation_replay_independently(self):
        fixture, observation = self.read('fixture.json'), self.read('observation.json')
        campaign = self.read('campaign-pins.json')
        pin = next(row for row in campaign['pins'] if row['client'] == 'claude')
        sessions = set()
        previous_finish = None
        self.assertEqual(len(campaign['pins']), 4)
        self.assertEqual({row['client'] for row in campaign['pins']}, set(PINS))
        for phase in ('positive', 'baseline', 'candidate'):
            with self.subTest(phase=phase):
                receipt, events = self.read(phase + '-receipt.json'), self.read(phase + '-native-selection.json')
                version = self.read(phase + '-version-receipt.json')
                started = datetime.datetime.fromisoformat(receipt['started_at_utc'])
                finished = datetime.datetime.fromisoformat(receipt['finished_at_utc'])
                self.assertEqual(started.tzinfo, datetime.timezone.utc)
                self.assertEqual(finished.tzinfo, datetime.timezone.utc)
                self.assertLess(started, finished)
                if previous_finish is not None:
                    self.assertLess(previous_finish, started)
                else:
                    for probe in campaign['pins']:
                        self.assertLess(datetime.datetime.fromisoformat(probe['finished_at_utc']), started)
                previous_finish = finished
                project = fixture['positive_project'] if phase == 'positive' else fixture['candidate_project']
                package = fixture['foreign_package'] if phase == 'candidate' else fixture['native_package']
                self.assertEqual(receipt['cwd'], project)
                self.assertEqual(version['stdout_sha256'], pin['stdout_sha256'])
                self.assertEqual(version['exit_code'], 0)
                self.assertLessEqual(version['finished_at_utc'], receipt['started_at_utc'])
                if phase == 'baseline':
                    self.assertNotIn('--plugin-dir', receipt['command'])
                else:
                    index = receipt['command'].index('--plugin-dir')
                    self.assertEqual(receipt['command'][index + 1], package)
                self.assertNotIn(fixture['marker'], receipt['command'][-1])
                context = dict(fixture, positive_project=project, native_package=package)
                parsed = q.inspect_cross_client_positive(events, context, exit_code=receipt['exit_code'], timed_out=receipt['timed_out'])
                self.assertEqual(parsed, observation[phase])
                self.assertEqual(parsed['state'], 'success')
                self.assertTrue(parsed['invocation_complete'])
                self.assertFalse(parsed['unexpected_tools'])
                self.assertEqual(parsed['invocation_outcome'], 'not_found' if phase == 'baseline' else 'loaded')
                self.assertEqual(parsed['activations'], 0 if phase == 'baseline' else 1)
                self.assertEqual(parsed['response'], fixture['absent_marker'] if phase == 'baseline' else fixture['marker'])
                init = next(event for event in events if event.get('subtype') == 'init')
                result = next(event for event in events if event['type'] == 'result')
                self.assertTrue(result['session_id'])
                self.assertEqual(init['session_id'], result['session_id'])
                sessions.add(result['session_id'])
        self.assertEqual(len(sessions), 3)
        self.assertEqual(fixture['packages'], observation['packages_after'])
        self.assertEqual(fixture['trees']['native'], fixture['trees']['foreign'])
        self.assertEqual(q.classify_cross_client_isolation(observation, fixture),
                         {'isolation': 'activation_observed', 'preservation': 'unchanged'})
        self.assertEqual(self.read('classification.json'), q.classify_cross_client_isolation(observation, fixture))

    def test_filtered_bookkeeping_witness_and_exact_restoration_receipts(self):
        fixture, witness = self.read('fixture.json'), self.read('bookkeeping-witness.json')
        preservation = self.read('preservation.json')
        self.assertEqual(preservation['before'], preservation['after_restoration'])
        self.assertIs(preservation['measured_restored'], True)
        self.assertIs(preservation['whole_profile_equal'], False)
        before = {'unchanged_fields_sha256': witness['other_fields_before_sha256'],
                  'cachedGrowthBookFeaturesAt': witness['cached_timestamp_before']}
        after = {'unchanged_fields_sha256': witness['other_fields_after_sha256'],
                 'cachedGrowthBookFeaturesAt': witness['cached_timestamp_after']}
        for field, expected_key in [('pluginUsage', fixture['plugin_name'] + '@inline'), ('skillUsage', fixture['namespace'])]:
            selected = witness['usage'][field]
            self.assertEqual(selected['owned_key'], expected_key)
            self.assertIsNone(selected['owned_before'])
            before[field] = {'unchanged_entries_sha256': selected['other_before_sha256']}
            after[field] = {'unchanged_entries_sha256': selected['other_after_sha256'], expected_key: selected['owned_after']}
        global_path = next(path for path in preservation['before'] if path.endswith('/.claude/.claude.json'))
        self.assertEqual(q.classify_claude_inline_bookkeeping(
            json.dumps(before).encode(), json.dumps(after).encode(),
            preservation['before'][global_path]['mode'], preservation['after_native_cleanup'][global_path]['mode'],
            plugin_name=fixture['plugin_name'], namespace=fixture['namespace']), 'native_bookkeeping_residue')
        self.assertEqual(preservation['residue'][global_path], 'native_bookkeeping_residue')

    def test_later_authentication_diagnostic_keeps_roots_and_profile_state(self):
        diagnostic = self.read('auth-diagnostic-repeat.json')
        self.assertEqual(diagnostic['differing_environment_keys'], ['USER'])
        self.assertIs(diagnostic['other_environment_values_equal'], True)
        self.assertEqual(diagnostic['model_requests'], 0)
        self.assertIs(diagnostic['raw_auth_output_retained'], False)
        self.assertEqual(diagnostic['profile_roots'], {
            'HOME': '$CLAUDE_PROFILE_ROOT/home', 'USERPROFILE': '$CLAUDE_PROFILE_ROOT/home',
            'CLAUDE_CONFIG_DIR': '$CLAUDE_PROFILE_ROOT/home/.claude'})
        self.assertEqual(diagnostic['measured_before'], diagnostic['measured_after'])
        self.assertEqual(diagnostic['measured_before'], self.read('preservation.json')['after_restoration'])
        probes = diagnostic['probes']
        self.assertEqual(len(probes), 3)
        previous_finish = datetime.datetime.fromisoformat(self.read('candidate-receipt.json')['finished_at_utc'])
        for probe, expected, exit_code in zip(probes, [True, False, True], [0, 1, 0]):
            with self.subTest(probe=probe['label']):
                self.assertIs(probe['USER_present'], expected)
                self.assertIs(probe['logged_in'], expected)
                self.assertEqual(probe['exit_code'], exit_code)
                self.assertEqual(probe['command'], ['$CLIENT_BIN/claude', 'auth', 'status'])
                started = datetime.datetime.fromisoformat(probe['started_at_utc'])
                finished = datetime.datetime.fromisoformat(probe['finished_at_utc'])
                self.assertLess(previous_finish, started)
                self.assertLess(started, finished)
                previous_finish = finished
