"""Offline tests for the separately versioned Copilot native-name calibration."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class CopilotNativeName(unittest.TestCase):
    def setUp(self):
        self.skill = b'---\nname: probe\ndescription: Test fixture.\n---\nReturn exactly PRIVATE and nothing else.\n'
        self.fixture = dict(client='copilot', pin='1.0.103', scope='session',
                            positive_project='/neutral/project', native_package='/neutral/plugin',
                            plugin_name='package', plugin_version='0.0.0', skill_name='probe', namespace='package:probe',
                            marker='PRIVATE', skill_file_sha256=hashlib.sha256(self.skill).hexdigest())
        def event(kind, identity, parent, data):
            return dict(type=kind, id=identity, parentId=parent, data=data)
        self.native = [
            event('session.start', 'root', None, dict(sessionId='session', copilotVersion='1.0.103', context={'cwd': '/neutral/project'})),
            event('user.message', 'user', 'root', dict(content='Invoke the skill named probe.', turnId='0')),
            event('tool.execution_start', 'call', 'user', dict(toolCallId='tool', toolName='skill', arguments={'skill': 'probe'}, turnId='0')),
            event('tool.execution_complete', 'done', 'call', dict(toolCallId='tool', success=True, turnId='0')),
            event('skill.invoked', 'load', 'done', dict(name='probe', path='/neutral/plugin/skills/probe/SKILL.md', source='plugin', pluginName='package', pluginVersion='0.0.0', content=self.skill.decode().split('\n---\n', 1)[1])),
            event('assistant.message', 'answer', 'load', dict(content='PRIVATE', turnId='1')),
            event('session.shutdown', 'shutdown', 'answer', {}),
        ]
        self.events = [
            event('session.skills_loaded', 'catalog', 'root', dict(skills=[dict(name='probe', commandName='package:probe', path='/neutral/plugin/skills/probe/SKILL.md', enabled=True)])),
            *copy.deepcopy(self.native[1:4]), copy.deepcopy(self.native[5]),
            dict(type='result', sessionId='session', exitCode=0),
        ]

    def inspect(self, **kwargs):
        return q.inspect_copilot_native_positive(self.events, self.native, self.fixture,
                                                skill_bytes=self.skill, exit_code=0, **kwargs)

    def assert_blocked(self):
        row = self.inspect()
        self.assertFalse(row['calibration_passed'])
        self.assertFalse(row['provenance_verified'])

    def test_one_native_name_load_requires_bound_provenance(self):
        row = self.inspect()
        self.assertTrue(row['calibration_passed'])
        self.assertTrue(row['provenance_verified'])
        self.assertEqual(row['calibration_profile'], 'copilot-native-name-v1')
        self.assertEqual(row['activations'], 1)
        self.assertEqual(row['invocation_name'], 'probe')
        self.assertEqual(row['source_path'], '/neutral/plugin')
        self.assertTrue(row['session_bound'])
        self.assertEqual(row['response'], 'PRIVATE')
        self.assertFalse(row['unexpected_tools'])

    def test_wrong_plugin_path_body_or_source_blocks(self):
        for key, value in [('pluginName', 'other'), ('path', '/other/probe/SKILL.md'),
                           ('content', 'changed'), ('name', 'other'), ('source', 'personal'), ('pluginVersion', 'wrong')]:
            with self.subTest(key=key):
                original = copy.deepcopy(self.native)
                self.native[4]['data'][key] = value
                self.assert_blocked()
                self.native = original

    def test_untrusted_fixture_bytes_block(self):
        self.fixture['skill_file_sha256'] = 'a' * 64
        self.assert_blocked()

    def test_colliding_bare_names_and_duplicate_catalogs_block(self):
        entry = self.events[0]['data']['skills'][0]
        self.events[0]['data']['skills'].append(dict(entry, commandName='other:probe', path='/other/skills/probe/SKILL.md'))
        self.assert_blocked()
        self.events[0]['data']['skills'].pop()
        self.events.insert(0, copy.deepcopy(self.events[0]))
        self.assert_blocked()

    def test_catalog_must_match_identity_and_source(self):
        for key, value in [('path', '/other/SKILL.md'), ('commandName', 'other:probe'),
                           ('name', 'other'), ('enabled', False)]:
            with self.subTest(key=key):
                before = copy.deepcopy(self.events)
                self.events[0]['data']['skills'][0][key] = value
                self.assert_blocked()
                self.events = before

    def test_substituted_session_version_or_cwd_blocks(self):
        for key, value in [('sessionId', 'other'), ('copilotVersion', 'wrong'), ('context', {'cwd': '/elsewhere'})]:
            with self.subTest(key=key):
                before = copy.deepcopy(self.native)
                self.native[0]['data'][key] = value
                self.assert_blocked()
                self.native = before

    def test_correlated_call_completion_and_load_are_required(self):
        for index in (2, 3, 4):
            with self.subTest(index=index):
                before = copy.deepcopy(self.native)
                self.native.pop(index)
                self.assert_blocked()
                self.native = before
        self.native[4]['parentId'] = 'user'
        self.assert_blocked()

    def test_duplicate_or_disconnected_native_records_block(self):
        self.native.insert(5, copy.deepcopy(self.native[4]))
        self.assert_blocked()
        self.native.pop(5)
        self.native[4]['parentId'] = 'missing'
        self.assert_blocked()

    def test_omitted_or_substituted_stdout_call_blocks(self):
        self.events[2]['data']['arguments']['skill'] = 'other'
        self.assert_blocked()
        self.events[2]['data']['arguments']['skill'] = 'probe'
        self.events.pop(2)
        self.assert_blocked()

    def test_failed_namespace_then_bare_name_is_not_relabelled(self):
        extra = copy.deepcopy(self.events[2]);extra['id'] = 'extra';extra['data']['toolCallId'] = 'extra';extra['data']['arguments']['skill'] = 'package:probe'
        self.events.insert(2, extra)
        self.assert_blocked()

    def test_load_must_precede_final_answer(self):
        self.native[4], self.native[5] = self.native[5], self.native[4]
        self.assert_blocked()

    def test_disclosure_or_extra_response_whitespace_blocks(self):
        self.native[1]['data']['content'] += ' PRIVATE'
        self.events[1] = copy.deepcopy(self.native[1])
        self.assert_blocked()
        self.native[1]['data']['content'] = 'Invoke the skill named probe.'
        self.events[1] = copy.deepcopy(self.native[1])
        self.events[4]['data']['content'] = self.native[5]['data']['content'] = '\n\nPRIVATE'
        self.assert_blocked()

    def test_terminal_error_timeout_or_malformed_capture_blocks(self):
        self.events[-1]['exitCode'] = 1
        self.assert_blocked()
        self.events[-1]['exitCode'] = 0
        self.assertFalse(self.inspect(timed_out=True)['calibration_passed'])
        self.assertFalse(self.inspect(malformed_stream=True)['calibration_passed'])
        for value in (None, {}, [None], [{'type': 'session.start', 'data': None}]):
            self.assertFalse(q.inspect_copilot_native_positive(value, value, self.fixture,
                             skill_bytes=self.skill, exit_code=0)['calibration_passed'])

    def test_malformed_native_metadata_blocks_without_exception(self):
        for index, key, value in [(0, 'context', None), (5, 'content', None)]:
            with self.subTest(index=index):
                before = copy.deepcopy(self.native)
                self.native[index]['data'][key] = value
                self.assert_blocked()
                self.native = before

    def test_catalog_alias_collision_and_nonboolean_enable_block(self):
        entry = self.events[0]['data']['skills'][0]
        self.events[0]['data']['skills'].append(dict(entry, name='other', commandName='probe', path='/other/SKILL.md'))
        self.assert_blocked()
        self.events[0]['data']['skills'].pop()
        self.events[0]['data']['skills'][0]['enabled'] = 1
        self.assert_blocked()

    def test_unindexed_or_duplicate_stdout_identity_blocks(self):
        self.events[4]['id'] = 'unknown'
        self.assert_blocked()
        self.events[4]['id'] = 'answer'
        self.events[1]['id'] = 'answer'
        self.assert_blocked()

    def test_malformed_arguments_catalog_or_fixture_blocks(self):
        for index, key, value in [(2, 'arguments', None), (0, 'skills', [None])]:
            with self.subTest(index=index):
                before = copy.deepcopy(self.events)
                self.events[index]['data'][key] = value
                self.assert_blocked()
                self.events = before
        del self.fixture['scope']
        self.assert_blocked()

    def test_stdout_order_and_response_ancestry_must_follow_load(self):
        self.events[2], self.events[3] = self.events[3], self.events[2]
        self.assert_blocked()
        self.events[2], self.events[3] = self.events[3], self.events[2]
        self.native[5]['parentId'] = self.events[4]['parentId'] = 'root'
        self.assert_blocked()

    def test_load_before_user_request_cannot_supply_calibration(self):
        user = self.native.pop(1)
        self.native.insert(4, user)
        self.native[1]['parentId'] = 'root'
        user['parentId'] = 'load'
        self.native[5]['parentId'] = 'user'
        self.events = [self.events[0], *copy.deepcopy([self.native[1], self.native[2], user, self.native[5]]), self.events[-1]]
        self.assert_blocked()

    def test_missing_native_data_is_rejected_without_exception(self):
        for index in range(len(self.native)):
            with self.subTest(index=index):
                before = copy.deepcopy(self.native)
                del self.native[index]['data']
                self.assert_blocked()
                self.native = before

    def test_retained_native_projection_replays_only_under_new_profile(self):
        packet = Path(__file__).parents[2] / 'docs/technotes/evidence/2026-10-30-q11b-native-name'
        load = lambda name: json.loads((packet / name).read_text())
        fixture = load('replay-fixture.json')
        events, native = load('stdout-selection.json'), load('native-selection.json')
        row = q.inspect_copilot_native_positive(events, native, fixture,
                skill_bytes=(packet / 'replay-skill.txt').read_bytes(), exit_code=0)
        self.assertEqual(row, load('replay-observation.json'))
        self.assertTrue(row['calibration_passed'])
        legacy = q.inspect_cross_client_positive(events, fixture, exit_code=0)
        self.assertEqual(legacy['activations'], 0)
        self.assertTrue(legacy['unexpected_tools'])
        invocation = next(e for e in native if e['type'] == 'skill.invoked')
        invocation['data']['pluginName'] = 'other-plugin'
        wrong = q.inspect_copilot_native_positive(events, native, fixture,
                skill_bytes=(packet / 'replay-skill.txt').read_bytes(), exit_code=0)
        self.assertFalse(wrong['calibration_passed'])

    def test_consistent_wrong_native_call_identity_blocks(self):
        for key, value in [('arguments', {'skill': 'other'}),
                           ('arguments', {'skill': 'package:probe'}), ('toolName', 'bash')]:
            with self.subTest(key=key, value=value):
                before_events, before_native = copy.deepcopy(self.events), copy.deepcopy(self.native)
                self.events[2]['data'][key] = self.native[2]['data'][key] = value
                self.assert_blocked()
                self.events, self.native = before_events, before_native

    def test_native_session_error_blocks_otherwise_consistent_capture(self):
        self.native.insert(5, dict(type='session.error', id='error', parentId='load', data={'message': 'provider error'}))
        self.native[6]['parentId'] = self.events[4]['parentId'] = 'error'
        self.assert_blocked()

    def test_assistant_marker_before_load_blocks(self):
        disclosure = dict(type='assistant.message', id='disclosure', parentId='user',
                          data={'content': 'PRIVATE', 'turnId': '0'})
        self.native[2]['parentId'] = self.events[2]['parentId'] = 'disclosure'
        self.native.insert(2, disclosure)
        self.events.insert(2, copy.deepcopy(disclosure))
        self.assert_blocked()

    def test_unhashable_parent_or_event_type_blocks_without_exception(self):
        for key in ('parentId', 'type'):
            for value in ([], {}, 1, None):
                with self.subTest(key=key, value=value):
                    before = copy.deepcopy(self.native)
                    self.native[2][key] = value
                    self.assert_blocked()
                    self.native = before

    def test_legacy_namespace_parser_remains_unchanged(self):
        row = q.inspect_cross_client_positive(self.events, self.fixture, exit_code=0)
        self.assertEqual(row['activations'], 0)
        self.assertTrue(row['unexpected_tools'])


if __name__ == '__main__':
    unittest.main()
