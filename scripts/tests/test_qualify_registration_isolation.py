"""Offline Q11b controls: no native process is started by this module."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class RegistrationIsolation(unittest.TestCase):
    def setUp(self):
        self.fixture = dict(owner='claude', observer='copilot', scope='project',
                            pins={'claude': '2.1.308', 'copilot': '1.0.103'},
                            cwd='/neutral/project', namespace='package:probe',
                            marker='PRESENT_private', absent_marker='ABSENT_private',
                            aliases=['probe'],
                            protected_paths=['config'],
                            source_paths={'owner_positive': '/neutral/claude-package',
                                          'observer_positive': '/neutral/copilot-package',
                                          'owner_candidate': '/neutral/claude-package'},
                            packages={'claude': {'.claude-plugin/plugin.json': {'sha256': 'd' * 64, 'mode': 0o644},
                                                 'skills/probe/SKILL.md': {'sha256': 'a' * 64, 'mode': 0o644},
                                                 'skills/probe/resource.txt': {'sha256': 'c' * 64, 'mode': 0o644}},
                                      'copilot': {'plugin.json': {'sha256': 'e' * 64, 'mode': 0o644},
                                                  'skills/probe/SKILL.md': {'sha256': 'a' * 64, 'mode': 0o644},
                                                  'skills/probe/resource.txt': {'sha256': 'c' * 64, 'mode': 0o644}}})
        self.fixture['skill_trees'] = {'claude': {'probe/SKILL.md': {'sha256': 'a' * 64, 'mode': 0o644},
                                                 'probe/resource.txt': {'sha256': 'c' * 64, 'mode': 0o644}},
                                       'copilot': {'probe/SKILL.md': {'sha256': 'a' * 64, 'mode': 0o644},
                                                  'probe/resource.txt': {'sha256': 'c' * 64, 'mode': 0o644}}}
        self.observation = dict(phases={}, operations={}, inventory={
            'complete': True, 'blocking_reasons': [],
            'owner': 'claude', 'observer': 'copilot', 'cwd': '/neutral/project',
            'scope': 'project', 'pins': self.fixture['pins'].copy(),
            'effective_roots': ['/neutral/home/.claude', '/neutral/home/.copilot'],
            'reader_conditions': ['No configured broad imports'],
            'surfaces': ['Copilot CLI only; IDE/cloud outside this claim']},
            packages_after=copy.deepcopy(self.fixture['packages']),
            skill_trees_after=copy.deepcopy(self.fixture['skill_trees']),
            preservation={'before': {'config': {'present': True, 'sha256': 'b' * 64, 'mode': 0o640}},
                          'after': {'config': {'present': True, 'sha256': 'b' * 64, 'mode': 0o640}}})
        timings = {'observer_positive': (1, 2), 'owner_positive': (3, 4),
                   'baseline': (7, 8), 'owner_candidate': (11, 12),
                   'candidate': (13, 14), 'post_removal': (17, 18)}
        for phase, (start, end) in timings.items():
            owner = phase in ('owner_positive', 'owner_candidate')
            positive = owner or phase == 'observer_positive'
            self.observation['phases'][phase] = dict(
                client='claude' if owner else 'copilot',
                pin='2.1.308' if owner else '1.0.103',
                scope='session' if phase == 'observer_positive' else 'project',
                cwd='/neutral/project', namespace='package:probe',
                session_id='session-' + phase, started_at=self.time(start), finished_at=self.time(end),
                state='success', complete=True, catalog_complete=True,
                catalog_present=positive, invocation_complete=True,
                invocation_attempted=True, invocation_outcome='loaded' if positive else 'not_found',
                activations=1 if positive else 0, content_activations=1 if positive else 0,
                response='PRESENT_private' if positive else 'ABSENT_private',
                unexpected_tools=False, foreign_selector=False,
                candidate_registered=owner, aliases_checked=['probe'],
                alias_activation=False, alias_exposure=False, alias_not_found=True,
                session_bound=True, marker_disclosed=False)
            if positive:
                self.observation['phases'][phase]['source_path'] = self.fixture['source_paths'][phase]
        for name, (start, end) in {'calibration_cleanup': (5, 6), 'owner_install': (9, 10),
                                  'owner_remove': (15, 16)}.items():
            self.observation['operations'][name] = dict(
                state='success', complete=True, started_at=self.time(start), finished_at=self.time(end),
                cwd='/neutral/project', owner='claude', scope='project', namespace='package:probe')

    def time(self, second):
        return f'2026-10-27T00:00:{second:02d}+00:00'

    def classify(self, value=None):
        return q.classify_registration_isolation(self.observation if value is None else value, self.fixture)

    def test_complete_cell_has_bounded_discovery_result(self):
        self.assertTrue(callable(getattr(q, 'classify_registration_isolation', None)),
                        'Q11b needs its own registration classifier before native execution')
        self.assertEqual(self.classify(), {'isolation': 'discovery_absence_observed', 'preservation': 'unchanged'})

    def test_each_native_positive_blocks_on_untrimmed_response_or_session_binding(self):
        for phase in ('owner_positive', 'observer_positive', 'owner_candidate'):
            for key, value in [('response', '\n\nPRESENT_private'), ('session_bound', False),
                               ('invocation_outcome', 'not_found'), ('activations', 0),
                               ('catalog_present', False), ('marker_disclosed', True)]:
                with self.subTest(phase=phase, key=key):
                    changed = copy.deepcopy(self.observation)
                    changed['phases'][phase][key] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'blocked_' + phase)

    def test_observer_sessions_must_be_fresh_distinct_and_ordered_after_operations(self):
        for phase, operation in [('baseline', 'calibration_cleanup'), ('candidate', 'owner_install'),
                                 ('post_removal', 'owner_remove')]:
            for key, value in [('session_id', 'session-observer_positive'),
                               ('started_at', self.observation['operations'][operation]['started_at']),
                               ('finished_at', 'invalid'), ('session_bound', False)]:
                with self.subTest(phase=phase, key=key):
                    changed = copy.deepcopy(self.observation)
                    changed['phases'][phase][key] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'blocked_freshness')

    def test_operation_failure_missing_metadata_and_reordered_effects_block(self):
        for name in self.observation['operations']:
            for key, value in [('complete', False), ('owner', 'other'), ('cwd', '/other'), ('scope', 'user'),
                               ('namespace', 'other:probe'), ('started_at', self.time(0))]:
                with self.subTest(operation=name, key=key):
                    changed = copy.deepcopy(self.observation)
                    changed['operations'][name][key] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'blocked_operations')

    def test_inventory_must_be_complete_bound_and_nonempty(self):
        for key, value in [('complete', False), ('blocking_reasons', ['unknown reader']),
                           ('effective_roots', []), ('reader_conditions', []), ('surfaces', []),
                           ('cwd', '/other'), ('pins', {}), ('scope', 'user')]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.observation)
                changed['inventory'][key] = value
                self.assertEqual(self.classify(changed)['isolation'], 'blocked_inventory')

    def test_foreign_selection_or_registration_cannot_supply_q11b_negative(self):
        for phase in ('baseline', 'candidate', 'post_removal'):
            for key in ('foreign_selector', 'candidate_registered', 'marker_disclosed'):
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase][key] = True
                self.assertEqual(self.classify(changed)['isolation'], 'invalid_observer_conditions')

    def test_baseline_contamination_stops_candidate_classification(self):
        for key, value in [('catalog_present', True), ('content_activations', 1),
                           ('alias_activation', True), ('response', 'prefix PRESENT_private')]:
            changed = copy.deepcopy(self.observation)
            changed['phases']['baseline'][key] = value
            self.assertEqual(self.classify(changed)['isolation'], 'baseline_contaminated')

    def test_activation_precedes_incomplete_catalog_or_failed_post_removal(self):
        for phase in ('candidate', 'post_removal'):
            for key, value in [('activations', 1), ('content_activations', 1),
                               ('alias_activation', True), ('response', 'prefix PRESENT_private')]:
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase].update(catalog_complete=False, **{key: value})
                self.assertEqual(self.classify(changed)['isolation'], 'activation_observed')

    def test_exposure_is_blocked_and_missing_alias_checks_cannot_pass(self):
        for phase in ('candidate', 'post_removal'):
            for key, value, expected in [('catalog_present', True, 'catalog_exposure'),
                                         ('alias_exposure', True, 'catalog_exposure'),
                                         ('aliases_checked', [], 'incomplete_evidence'),
                                         ('alias_not_found', False, 'incomplete_evidence'),
                                         ('invocation_outcome', 'not_attempted', 'incomplete_evidence')]:
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase][key] = value
                self.assertEqual(self.classify(changed)['isolation'], expected)

    def test_package_edits_and_bad_phase_identity_block(self):
        changed = copy.deepcopy(self.observation)
        changed['packages_after']['claude']['skills/probe/SKILL.md']['mode'] = 0o600
        self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')
        for field in ('packages_after', 'skill_trees_after'):
            changed = copy.deepcopy(self.observation)
            changed[field] = {}
            self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')
        self.fixture['skill_trees']['copilot']['probe/resource.txt']['mode'] = 0o600
        self.observation['skill_trees_after'] = copy.deepcopy(self.fixture['skill_trees'])
        self.assertEqual(self.classify()['isolation'], 'invalid_pair')
        self.fixture['skill_trees']['copilot']['probe/resource.txt']['mode'] = 0o644
        self.observation['skill_trees_after'] = copy.deepcopy(self.fixture['skill_trees'])
        for phase in self.observation['phases']:
            for key in ('pin', 'cwd', 'namespace', 'scope', 'client'):
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase][key] = 'wrong'
                self.assertEqual(self.classify(changed)['isolation'], 'control_metadata_failed')

    def test_preservation_is_independent_and_requires_valid_complete_receipts(self):
        for value in ({}, {'before': {}, 'after': {}},
                      {'before': {'config': {'present': True}}, 'after': {'config': {'present': True}}}):
            changed = copy.deepcopy(self.observation)
            changed['preservation'] = value
            self.assertEqual(self.classify(changed), {'isolation': 'discovery_absence_observed',
                                                     'preservation': 'preservation_failed'})
        changed = copy.deepcopy(self.observation)
        changed['preservation']['after']['config']['mode'] = 0o600
        self.assertEqual(self.classify(changed)['preservation'], 'preservation_failed')

    def test_positive_requires_expected_source_and_correlated_content_activation(self):
        for phase in ('observer_positive', 'owner_positive', 'owner_candidate'):
            for key, value in [('source_path', None), ('source_path', '/unrelated/package'),
                               ('content_activations', 0), ('content_activations', 2)]:
                with self.subTest(phase=phase, key=key, value=value):
                    changed = copy.deepcopy(self.observation)
                    changed['phases'][phase][key] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'blocked_' + phase)

    def test_rejects_malformed_trees_receipts_and_contradictory_envelopes(self):
        for value in ('not-a-tree', [], {}, {'probe/SKILL.md': {}},
                      {'probe/SKILL.md': {'sha256': 'z' * 64, 'mode': 0o644}},
                      {'../escape': {'sha256': 'a' * 64, 'mode': 0o644}},
                      {'probe/SKILL.md': {'sha256': 'a' * 64, 'mode': True}}):
            with self.subTest(value=value):
                original = copy.deepcopy(self.fixture['skill_trees'])
                self.fixture['skill_trees'] = {'claude': value, 'copilot': value}
                changed = copy.deepcopy(self.observation)
                changed['skill_trees_after'] = copy.deepcopy(self.fixture['skill_trees'])
                self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')
                self.fixture['skill_trees'] = original
        for value in ({}, 'not-a-tree', {'plugin.json': {'sha256': 'a' * 64, 'mode': -1}}):
            original = copy.deepcopy(self.fixture['packages'])
            self.fixture['packages']['claude'] = value
            changed = copy.deepcopy(self.observation)
            changed['packages_after'] = copy.deepcopy(self.fixture['packages'])
            self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')
            self.fixture['packages'] = original

    def test_owner_positives_require_affirmative_registration(self):
        for phase in ('owner_positive', 'owner_candidate'):
            for value in (False, None, 'unknown', 1):
                with self.subTest(phase=phase, value=value):
                    changed = copy.deepcopy(self.observation)
                    changed['phases'][phase]['candidate_registered'] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'blocked_' + phase)

    def test_preservation_requires_exact_declared_path_coverage(self):
        for paths in (None, [], ['config', 'config'], ['config', 'missing'], [None]):
            with self.subTest(paths=paths):
                fixture = copy.deepcopy(self.fixture)
                fixture['protected_paths'] = paths
                self.assertEqual(q.classify_registration_isolation(self.observation, fixture)['preservation'],
                                 'preservation_failed')
        for receipts in ({'unrelated': {'present': False}},
                         {'config': {'present': False}, 'extra': {'present': False}}):
            changed = copy.deepcopy(self.observation)
            changed['preservation'] = dict(before=receipts, after=copy.deepcopy(receipts))
            self.assertEqual(self.classify(changed)['preservation'], 'preservation_failed')
        for paths, receipts in [([], {}), ([None], {None: {'present': False}}),
                                ([''], {'': {'present': False}})]:
            fixture = copy.deepcopy(self.fixture)
            fixture['protected_paths'] = paths
            changed = copy.deepcopy(self.observation)
            changed['preservation'] = dict(before=receipts, after=copy.deepcopy(receipts))
            self.assertEqual(q.classify_registration_isolation(changed, fixture)['preservation'], 'preservation_failed')
        fixture = copy.deepcopy(self.fixture)
        fixture['protected_paths'] = ['config', 'other']
        changed = copy.deepcopy(self.observation)
        changed['preservation']['before']['other'] = {'present': False}
        changed['preservation']['after']['other'] = {'present': False}
        self.assertEqual(q.classify_registration_isolation(changed, fixture)['preservation'], 'unchanged')

    def test_candidate_activation_survives_later_cleanup_and_observation_gaps(self):
        for value in (None, {}, [], 'unknown'):
            for section in ('post_removal', 'owner_remove'):
                with self.subTest(section=section, value=value):
                    changed = copy.deepcopy(self.observation)
                    changed['phases']['candidate']['activations'] = 1
                    changed['phases' if section == 'post_removal' else 'operations'][section] = value
                    self.assertEqual(self.classify(changed)['isolation'], 'activation_observed')
        for key in ('client', 'pin', 'cwd', 'scope', 'namespace', 'started_at', 'finished_at'):
            changed = copy.deepcopy(self.observation)
            changed['phases']['candidate']['activations'] = 1
            changed['phases']['post_removal'][key] = 'invalid-later-observation'
            self.assertEqual(self.classify(changed)['isolation'], 'activation_observed')
        changed = copy.deepcopy(self.observation)
        changed['phases']['candidate']['activations'] = 1
        changed['phases']['post_removal']['session_id'] = 'session-observer_positive'
        self.assertEqual(self.classify(changed)['isolation'], 'activation_observed')
        for key, value in [('complete', False), ('state', 'error'), ('cwd', '/other')]:
            changed = copy.deepcopy(self.observation)
            changed['phases']['candidate']['activations'] = 1
            changed['operations']['owner_remove'][key] = value
            self.assertEqual(self.classify(changed)['isolation'], 'activation_observed')
        for preservation in (None, [], {},
                             {'before': {'config': {'present': True, 'sha256': 'b' * 64, 'mode': 0o640}},
                              'after': {'config': {'present': True, 'sha256': 'b' * 64, 'mode': 0o600}}}):
            changed = copy.deepcopy(self.observation)
            changed['phases']['candidate']['activations'] = 1
            changed['preservation'] = preservation
            self.assertEqual(self.classify(changed), {'isolation': 'activation_observed',
                                                     'preservation': 'preservation_failed'})

    def test_later_package_integrity_cannot_erase_candidate_activation(self):
        for field in ('packages_after', 'skill_trees_after'):
            for value in (None, {}, [], 'unknown'):
                with self.subTest(field=field, value=value):
                    changed = copy.deepcopy(self.observation)
                    changed['phases']['candidate']['activations'] = 1
                    changed[field] = value
                    self.assertEqual(self.classify(changed), {'isolation': 'activation_observed',
                                                             'preservation': 'preservation_failed'})
        for field, path in [('packages_after', 'skills/probe/resource.txt'),
                            ('skill_trees_after', 'probe/resource.txt')]:
            for client in ('claude', 'copilot'):
                for key, value in [('sha256', 'f' * 64), ('mode', 0o600)]:
                    changed = copy.deepcopy(self.observation)
                    changed['phases']['candidate']['activations'] = 1
                    changed[field][client][path][key] = value
                    self.assertEqual(self.classify(changed), {'isolation': 'activation_observed',
                                                             'preservation': 'preservation_failed'})

    def test_untrusted_candidate_activation_does_not_bypass_earlier_controls(self):
        for section, key, value, expected in [
                ('observer_positive', 'complete', False, 'blocked_observer_positive'),
                ('owner_candidate', 'candidate_registered', False, 'blocked_owner_candidate'),
                ('baseline', 'response', 'unrelated', 'blocked_baseline'),
                ('candidate', 'session_bound', False, 'blocked_freshness'),
                ('candidate', 'foreign_selector', True, 'invalid_observer_conditions')]:
            changed = copy.deepcopy(self.observation)
            changed['phases']['candidate']['activations'] = 1
            changed['phases'][section][key] = value
            self.assertEqual(self.classify(changed)['isolation'], expected)

    def test_malformed_capture_containers_block_without_exceptions(self):
        for value in (None, [], 'unknown', 1):
            with self.subTest(top_level=value):
                self.assertEqual(q.classify_registration_isolation(value, self.fixture)['isolation'], 'invalid_capture')
                self.assertEqual(q.classify_registration_isolation(self.observation, value)['isolation'], 'invalid_fixture')
            for section, expected in [('inventory', 'blocked_inventory'), ('phases', 'blocked_observer_positive'),
                                      ('operations', 'blocked_operations')]:
                changed = copy.deepcopy(self.observation)
                changed[section] = value
                self.assertEqual(self.classify(changed)['isolation'], expected)
                self.assertEqual(self.classify(changed)['preservation'], 'unchanged')
            changed = copy.deepcopy(self.observation)
            changed['preservation'] = value
            self.assertEqual(self.classify(changed), {'isolation': 'discovery_absence_observed',
                                                     'preservation': 'preservation_failed'})
            for phase in self.observation['phases']:
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase] = value
                expected = ('blocked_' + phase if phase in ('observer_positive', 'owner_positive', 'owner_candidate')
                            else 'blocked_freshness')
                self.assertEqual(self.classify(changed)['isolation'], expected)
            for operation in self.observation['operations']:
                changed = copy.deepcopy(self.observation)
                changed['operations'][operation] = value
                self.assertEqual(self.classify(changed)['isolation'], 'blocked_operations')

    def test_malformed_fixture_identity_and_containers_block(self):
        for key in ('owner', 'observer', 'scope', 'cwd', 'namespace', 'marker', 'absent_marker',
                    'pins', 'source_paths', 'aliases'):
            for value in (None, 1):
                with self.subTest(key=key, value=value):
                    fixture = copy.deepcopy(self.fixture)
                    fixture[key] = value
                    self.assertEqual(q.classify_registration_isolation(self.observation, fixture),
                                     {'isolation': 'invalid_fixture', 'preservation': 'preservation_failed'})
        for aliases in ([None], [''], [1], ['probe', 'probe']):
            fixture = copy.deepcopy(self.fixture)
            fixture['aliases'] = aliases
            changed = copy.deepcopy(self.observation)
            for phase in ('baseline', 'candidate', 'post_removal'):
                changed['phases'][phase]['aliases_checked'] = aliases.copy()
            self.assertEqual(q.classify_registration_isolation(changed, fixture)['isolation'], 'invalid_fixture')
        fixture = copy.deepcopy(self.fixture)
        fixture.update(observer='claude', pins={'claude': '2.1.308'})
        fixture['packages'] = {'claude': fixture['packages']['claude']}
        fixture['skill_trees'] = {'claude': fixture['skill_trees']['claude']}
        changed = copy.deepcopy(self.observation)
        changed['inventory'].update(observer='claude', pins=fixture['pins'].copy())
        changed['packages_after'] = copy.deepcopy(fixture['packages'])
        changed['skill_trees_after'] = copy.deepcopy(fixture['skill_trees'])
        for phase in changed['phases'].values():
            phase.update(client='claude', pin='2.1.308')
        self.assertEqual(q.classify_registration_isolation(changed, fixture)['isolation'], 'invalid_fixture')
        for section in ('before', 'after'):
            for value in (None, [], 'unknown', 1):
                changed = copy.deepcopy(self.observation)
                changed['preservation'][section] = value
                self.assertEqual(self.classify(changed)['preservation'], 'preservation_failed')

    def test_phase_completion_and_tool_restrictions_are_independent_guards(self):
        for phase in self.observation['phases']:
            for key, value in [('state', 'error'), ('complete', False), ('catalog_complete', False),
                               ('invocation_complete', False), ('invocation_attempted', False),
                               ('unexpected_tools', True)]:
                with self.subTest(phase=phase, key=key):
                    changed = copy.deepcopy(self.observation)
                    changed['phases'][phase][key] = value
                    expected = ('blocked_' + phase if phase in ('observer_positive', 'owner_positive', 'owner_candidate')
                                else 'blocked_baseline' if phase == 'baseline' else 'incomplete_evidence')
                    self.assertEqual(self.classify(changed)['isolation'], expected)

    def test_operation_state_and_inventory_clients_are_bound(self):
        for operation in self.observation['operations']:
            for state in ('error', 'unknown', None):
                changed = copy.deepcopy(self.observation)
                changed['operations'][operation]['state'] = state
                self.assertEqual(self.classify(changed)['isolation'], 'blocked_operations')
        for client in ('owner', 'observer'):
            changed = copy.deepcopy(self.observation)
            changed['inventory'][client] = 'other'
            self.assertEqual(self.classify(changed)['isolation'], 'blocked_inventory')

    def test_absence_response_is_exact_in_every_negative_phase(self):
        for phase in ('baseline', 'candidate', 'post_removal'):
            for response in (None, '', 'unrelated', 'prefix ABSENT_private', '\nABSENT_private', 'ABSENT_private '):
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase]['response'] = response
                self.assertEqual(self.classify(changed)['isolation'],
                                 'blocked_baseline' if phase == 'baseline' else 'incomplete_evidence')

    def test_alias_identity_and_multiple_aliases_are_bound(self):
        self.fixture['aliases'] = ['probe', 'compat-probe']
        for phase in ('baseline', 'candidate', 'post_removal'):
            self.observation['phases'][phase]['aliases_checked'] = ['probe', 'compat-probe']
        self.assertEqual(self.classify()['isolation'], 'discovery_absence_observed')
        for phase in ('baseline', 'candidate', 'post_removal'):
            for aliases in (['wrong', 'compat-probe'], ['probe'], ['probe', 'wrong']):
                changed = copy.deepcopy(self.observation)
                changed['phases'][phase]['aliases_checked'] = aliases
                self.assertEqual(self.classify(changed)['isolation'],
                                 'blocked_baseline' if phase == 'baseline' else 'incomplete_evidence')

    def test_cross_client_inequality_with_consistent_envelopes_is_rejected(self):
        for key, value in [('mode', 0o600), ('sha256', 'f' * 64)]:
            fixture = copy.deepcopy(self.fixture)
            fixture['packages']['copilot']['skills/probe/resource.txt'][key] = value
            fixture['skill_trees']['copilot']['probe/resource.txt'][key] = value
            changed = copy.deepcopy(self.observation)
            changed['packages_after'] = copy.deepcopy(fixture['packages'])
            changed['skill_trees_after'] = copy.deepcopy(fixture['skill_trees'])
            self.assertEqual(q.classify_registration_isolation(changed, fixture)['isolation'], 'invalid_pair')

    def test_observer_only_post_run_changes_are_rejected(self):
        for field, path in [('packages_after', 'skills/probe/resource.txt'), ('skill_trees_after', 'probe/resource.txt')]:
            for key, value in [('sha256', 'f' * 64), ('mode', 0o600)]:
                changed = copy.deepcopy(self.observation)
                changed[field]['copilot'][path][key] = value
                self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')

    def test_consistent_invalid_tree_paths_digests_and_modes_are_rejected(self):
        for invalid_path in ('../escape', '/absolute', 'probe/../escape', 'probe//resource.txt'):
            fixture = copy.deepcopy(self.fixture)
            for client in ('claude', 'copilot'):
                receipt = fixture['skill_trees'][client].pop('probe/resource.txt')
                fixture['skill_trees'][client][invalid_path] = receipt
                del fixture['packages'][client]['skills/probe/resource.txt']
                fixture['packages'][client]['skills/' + invalid_path] = receipt.copy()
            changed = copy.deepcopy(self.observation)
            changed['packages_after'] = copy.deepcopy(fixture['packages'])
            changed['skill_trees_after'] = copy.deepcopy(fixture['skill_trees'])
            self.assertEqual(q.classify_registration_isolation(changed, fixture)['isolation'], 'invalid_pair')
        for key, value in [('sha256', 'z' * 64), ('sha256', 'a' * 63), ('mode', True), ('mode', -1), ('mode', 0o10000)]:
            fixture = copy.deepcopy(self.fixture)
            for client in ('claude', 'copilot'):
                fixture['packages'][client]['skills/probe/resource.txt'][key] = value
                fixture['skill_trees'][client]['probe/resource.txt'][key] = value
            changed = copy.deepcopy(self.observation)
            changed['packages_after'] = copy.deepcopy(fixture['packages'])
            changed['skill_trees_after'] = copy.deepcopy(fixture['skill_trees'])
            self.assertEqual(q.classify_registration_isolation(changed, fixture)['isolation'], 'invalid_pair')

    def test_timestamp_zone_and_owner_install_completion_are_required(self):
        changed = copy.deepcopy(self.observation)
        for section in ('phases', 'operations'):
            for row in changed[section].values():
                for key in ('started_at', 'finished_at'):
                    row[key] = row[key].replace('+00:00', '')
        self.assertEqual(self.classify(changed)['isolation'], 'blocked_freshness')
        changed = copy.deepcopy(self.observation)
        changed['phases']['owner_candidate']['started_at'] = self.time(9)
        self.assertEqual(self.classify(changed)['isolation'], 'blocked_freshness')

    def test_preservation_receipt_shapes_are_validated_independently(self):
        for row, expected in [({'present': False}, 'unchanged'),
                              ({'present': False, 'sha256': 'a' * 64}, 'preservation_failed'),
                              ({'present': False, 'mode': 0o600}, 'preservation_failed'),
                              ({'present': True, 'sha256': 'z' * 64, 'mode': 0o600}, 'preservation_failed'),
                              ({'present': True, 'sha256': 'a' * 64, 'mode': True}, 'preservation_failed'),
                              ({'present': True, 'sha256': 'a' * 64, 'mode': -1}, 'preservation_failed')]:
            changed = copy.deepcopy(self.observation)
            changed['preservation'] = dict(before={'config': row}, after={'config': row.copy()})
            self.assertEqual(self.classify(changed)['preservation'], expected)

    def test_inconsistent_package_skill_subtrees_are_rejected(self):
        for mutation in ('missing_resource', 'changed_resource', 'extra_skill'):
            original = copy.deepcopy(self.fixture['packages'])
            package = self.fixture['packages']['claude']
            if mutation == 'missing_resource':
                del package['skills/probe/resource.txt']
            elif mutation == 'changed_resource':
                package['skills/probe/resource.txt']['sha256'] = 'f' * 64
            else:
                package['skills/extra/SKILL.md'] = {'sha256': 'a' * 64, 'mode': 0o644}
            changed = copy.deepcopy(self.observation)
            changed['packages_after'] = copy.deepcopy(self.fixture['packages'])
            self.assertEqual(self.classify(changed)['isolation'], 'invalid_pair')
            self.fixture['packages'] = original


if __name__ == '__main__':
    unittest.main()
