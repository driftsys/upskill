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
            for key, value in [('complete', False), ('cwd', '/other'), ('scope', 'user'),
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
