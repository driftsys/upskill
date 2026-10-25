import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class ClaudeScopeOverlapTests(unittest.TestCase):
    def classify(self, controls, fixture):
        self.assertTrue(callable(getattr(q, 'classify_claude_scope_overlap', None)), 'Missing Q10 classifier')
        return q.classify_claude_scope_overlap(controls, fixture)

    def fixture(self):
        package = Path('/fixture/delivery/packages/fixture-package')
        body = 'Activation receipt: ACTIVATION_RECEIPT. Read references/receipt.txt once and return its single line.\n'
        return {'pin': '2.1.308', 'plugin_name': 'fixture-package', 'market_name': 'fixture-market',
                'skill_name': 'probe', 'package': package, 'project': Path('/fixture/project'),
                'resource': package / 'skills/probe/references/receipt.txt',
                'resource_marker': 'RESOURCE_RECEIPT', 'activation_marker': 'ACTIVATION_RECEIPT',
                'skill_body': body, 'fallback': 'ABSENT_RECEIPT'}

    def controls(self, f):
        namespace = f['plugin_name'] + ':' + f['skill_name']
        phases = {}
        scopes = {'project_positive': ['project'], 'user_positive': ['user'],
                  'overlap': ['user', 'project'], 'user_remaining': ['user'],
                  'project_remaining': ['project']}
        for phase, installed in scopes.items():
            sid = 'session-' + phase
            def event(kind, **values):
                return {'type': kind, 'session_id': sid, **values}
            phases[phase] = {
                'version': f['pin'], 'exit_code': 0, 'timed_out': False, 'malformed_stream': False, 'cwd': str(f['project']),
                'enabled_scopes': {scope: scope in installed for scope in ['user', 'project']},
                'bindings': [{'scope': scope, 'version': '0.0.0', 'installPath': '/native-cache/' + scope + '/fixture-package/0.0.0',
                              **({'projectPath': str(f['project'])} if scope == 'project' else {})} for scope in installed],
                'events': [
                    event('system', subtype='init', claude_code_version=f['pin'], cwd=str(f['project']), skills=[namespace], slash_commands=[namespace]),
                    event('assistant', message={'content': [{'type': 'tool_use', 'id': 'skill-call', 'name': 'Skill', 'input': {'skill': namespace}}]}),
                    event('user', message={'content': [{'type': 'tool_result', 'tool_use_id': 'skill-call', 'content': 'Launching skill: ' + namespace}]}),
                    event('user', message={'content': [{'type': 'text', 'text': 'Base directory for this skill: ' + str(f['package'] / 'skills' / f['skill_name']) + '\n\n' + f['skill_body']}]}),
                    event('assistant', message={'content': [{'type': 'tool_use', 'id': 'read-call', 'name': 'Read', 'input': {'file_path': str(f['resource'])}}]}),
                    event('user', message={'content': [{'type': 'tool_result', 'tool_use_id': 'read-call', 'content': '     1→' + f['resource_marker'] + '\n'}]}),
                    event('result', subtype='success', is_error=False, result=f['resource_marker'], permission_denials=[]),
                ],
            }
        return {'preserved': True, 'phases': phases}

    def test_fixture_reuses_neutral_marketplace_with_a_resource_only_response(self):
        self.assertTrue(callable(getattr(q, 'prepare_claude_scope_overlap', None)), 'Missing Q10 fixture')
        with tempfile.TemporaryDirectory() as tmp:
            f = q.prepare_claude_scope_overlap(Path(tmp) / 'fixture', pin='2.1.308')
            self.assertTrue(f['plugin_name'].startswith('upskill-q10-'))
            self.assertEqual(f['pin'], '2.1.308')
            self.assertEqual(f['root'].stat().st_mode & 0o777, 0o700)
            self.assertEqual(f['resource'].read_text(), f['resource_marker'] + '\n')
            self.assertNotIn(f['resource_marker'], f['skill'].read_text())
            self.assertIn(f['activation_marker'], f['skill_body'])
            self.assertIn(f['skill_body'], f['skill'].read_text())
            self.assertIn('Read references/receipt.txt exactly once', f['skill'].read_text())
            self.assertIn('Return only the single line read from that file.', f['skill'].read_text())
            self.assertNotIn(f['fallback'], f['skill'].read_text())
            self.assertEqual(len({f['resource_marker'], f['activation_marker'], f['fallback']}), 3)
            market = json.loads((f['managed_root'] / '.claude-plugin/marketplace.json').read_text())
            self.assertEqual(market['name'], f['market_name'])
            self.assertEqual(market['plugins'], [{'name': f['plugin_name'], 'source': './packages/' + f['plugin_name']}])
            self.assertEqual(json.loads((f['package'] / '.claude-plugin/plugin.json').read_text())['name'], f['plugin_name'])
            self.assertEqual(list(f['root'].rglob('SKILL.md')), [f['skill']])
            self.assertFalse(any(p.is_symlink() or p.name in ('CLAUDE.md', 'AGENTS.md', 'AGENTS.override.md') for p in f['root'].rglob('*')))
            self.assertFalse(f['managed_root'].is_relative_to(f['project']))
            self.assertEqual((f['project'] / '.claude/settings.json').stat().st_mode & 0o777, 0o640)

    def test_fixture_rejects_existing_discovery_roots_and_bad_pins_before_writes(self):
        self.assertTrue(callable(getattr(q, 'prepare_claude_scope_overlap', None)), 'Missing Q10 fixture')
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            for path in [base / '.claude/probe', base / '.agents/skills/probe']:
                with self.assertRaises(ValueError):q.prepare_claude_scope_overlap(path, pin='2.1.308')
                self.assertFalse(path.exists())
            for pin in [None, '', '2.1', 'not-a-release']:
                path = base / 'invalid-pin'
                with self.assertRaises(ValueError):q.prepare_claude_scope_overlap(path, pin=pin)
                self.assertFalse(path.exists())
            existing = base / 'existing';existing.mkdir();(existing / 'sentinel').write_bytes(b'preserve')
            with self.assertRaises(ValueError):q.prepare_claude_scope_overlap(existing, pin='2.1.308')
            self.assertEqual((existing / 'sentinel').read_bytes(), b'preserve')

    def test_both_calibrations_overlap_and_surviving_scopes_are_observed(self):
        f = self.fixture();c = self.controls(f)
        self.assertEqual(self.classify(c, f), 'observed_single_entry_and_activation')

    def test_failed_positives_precede_candidate_and_preservation_conclusions(self):
        f = self.fixture()
        for phase in ['project_positive', 'user_positive']:
            for key, value in [('exit_code', 1), ('timed_out', True)]:
                c = self.controls(f);c['phases'][phase][key] = value;c['preserved'] = False
                self.assertEqual(self.classify(c, f), 'positive_control_failed')
            c = self.controls(f);c['phases'][phase]['events'][-1]['is_error'] = True
            self.assertEqual(self.classify(c, f), 'positive_control_failed')
            c = self.controls(f);del c['phases'][phase]
            self.assertEqual(self.classify(c, f), 'positive_control_failed')

    def test_duplicate_catalog_or_body_expansion_is_not_hidden_by_one_skill_call(self):
        f = self.fixture();ns = f['plugin_name'] + ':' + f['skill_name']
        for field in ['skills', 'slash_commands']:
            for alias in [ns, f['skill_name']]:
                c = self.controls(f);c['phases']['overlap']['events'][0][field].append(alias)
                self.assertEqual(self.classify(c, f), 'duplicate_exposure_or_activation')
        c = self.controls(f);c['phases']['overlap']['events'][3]['message']['content'].append({'type': 'text', 'text': f['skill_body']})
        self.assertEqual(self.classify(c, f), 'duplicate_exposure_or_activation')
        c = self.controls(f);events=c['phases']['overlap']['events'];call=copy.deepcopy(events[1]);call['message']['content'][0]['id']='second-skill';result=copy.deepcopy(events[2]);result['message']['content'][0]['tool_use_id']='second-skill';events[4:4]=[call,result]
        self.assertEqual(self.classify(c, f), 'duplicate_exposure_or_activation')

    def test_missing_native_catalog_or_body_visibility_is_unobserved(self):
        f = self.fixture()
        for field in ['skills', 'slash_commands']:
            c = self.controls(f);del c['phases']['overlap']['events'][0][field]
            self.assertEqual(self.classify(c, f), 'activation_evidence_unobserved')
        c = self.controls(f);c['phases']['overlap']['events'][3]['message']['content']=[]
        self.assertEqual(self.classify(c, f), 'activation_evidence_unobserved')
        c = self.controls(f);c['phases']['overlap']['events'][3]['message']['content'][0]['text']=f['activation_marker']
        self.assertEqual(self.classify(c, f), 'activation_evidence_unobserved')

    def test_correlated_resource_reads_completion_and_native_identity_are_required(self):
        f = self.fixture()
        mutations = [
            lambda p:p.update(version='old'),
            lambda p:p.update(exit_code=False),
            lambda p:p['events'][-1].update(subtype='error_during_execution'),
            lambda p:p.update(cwd='/another/workspace'),
            lambda p:p['events'][0].update(cwd='/another/workspace'),
            lambda p:p['events'][1]['message']['content'][0]['input'].update(skill='another-package:probe'),
            lambda p:p['events'][1]['message']['content'][0]['input'].update(skill=f['skill_name']),
            lambda p:p['events'][0].update(claude_code_version='old'),
            lambda p:p['events'][-1].update(session_id='another-session'),
            lambda p:p['events'][-1].update(result=f['fallback']),
            lambda p:p['events'][-1].update(permission_denials=[{'tool_name':'Read'}]),
            lambda p:p['events'][4]['message']['content'][0]['input'].update(file_path='/unrelated/file'),
            lambda p:p['events'][5]['message']['content'][0].update(tool_use_id='unmatched'),
            lambda p:p['events'][5]['message']['content'][0].update(is_error=True),
            lambda p:p['events'][5]['message']['content'][0].update(content='NOT_'+f['resource_marker']),
            lambda p:p['events'][1]['message']['content'][0].update(name='Bash'),
        ]
        for change in mutations:
            c=self.controls(f);change(c['phases']['overlap'])
            self.assertEqual(self.classify(c, f), 'control_failed')
        c=self.controls(f);c['phases']['overlap']['events'].append(copy.deepcopy(c['phases']['overlap']['events'][-1]))
        self.assertEqual(self.classify(c, f), 'control_failed')

    def test_scope_bindings_and_enabled_values_cannot_be_assumed_from_catalog(self):
        f=self.fixture()
        for change in [lambda p:p.update(bindings=[]),lambda p:p['bindings'].append(copy.deepcopy(p['bindings'][0])),lambda p:p['enabled_scopes'].update(user=1),lambda p:p['bindings'][1].update(projectPath='/unrelated/project')]:
            c=self.controls(f);change(c['phases']['overlap'])
            self.assertEqual(self.classify(c, f),'registration_mismatch')
        c=self.controls(f);c['phases']['project_remaining']['enabled_scopes']['user']=True
        self.assertEqual(self.classify(c,f),'registration_mismatch')

    def test_all_phases_fresh_sessions_and_measured_preservation_are_required(self):
        f=self.fixture();c=self.controls(f);c['preserved']=False
        self.assertEqual(self.classify(c,f),'preservation_failed')
        for phase in ['overlap','user_remaining','project_remaining']:
            c=self.controls(f);del c['phases'][phase]
            self.assertEqual(self.classify(c,f),'control_incomplete')
        c=self.controls(f);sid=c['phases']['user_positive']['events'][0]['session_id']
        for e in c['phases']['overlap']['events']:e['session_id']=sid
        self.assertEqual(self.classify(c,f),'session_identity_mismatch')

    def test_one_phase_can_gate_the_next_native_scope_installation(self):
        self.assertTrue(callable(getattr(q, 'inspect_claude_scope_phase', None)), 'Missing Q10 phase inspector')
        f=self.fixture();row=self.controls(f)['phases']['project_positive']
        reason, evidence=q.inspect_claude_scope_phase(row,f,['project'])
        self.assertIsNone(reason)
        self.assertEqual(evidence['counts'],[1,1,1,1,1])
        reason, _=q.inspect_claude_scope_phase(row,f,['user'])
        self.assertEqual(reason,'registration_mismatch')
        for value in [None, False, 'unknown']:
            row=self.controls(f)['phases']['project_positive'];row['bindings'][0]['scope']=value
            self.assertEqual(q.inspect_claude_scope_phase(row,f,['project'])[0],'registration_mismatch')

    def test_observed_duplicate_is_retained_when_later_probes_are_not_run(self):
        f=self.fixture();c=self.controls(f)
        c['phases']['overlap']['events'][0]['skills'].append(f['skill_name'])
        del c['phases']['user_remaining'];del c['phases']['project_remaining']
        self.assertEqual(self.classify(c,f),'duplicate_exposure_or_activation')

    def test_each_native_scope_installation_can_supply_the_relative_resource(self):
        f=self.fixture()
        for scope in ['user','project']:
            c=self.controls(f);row=c['phases']['overlap']
            self.assertEqual(len({b['installPath'] for b in row['bindings']}),2)
            binding=next(b for b in row['bindings'] if b['scope']==scope)
            row['events'][4]['message']['content'][0]['input']['file_path']=str(Path(binding['installPath'])/'skills'/f['skill_name']/'references/receipt.txt')
            self.assertEqual(self.classify(c,f),'observed_single_entry_and_activation')

    def test_complete_body_plus_partial_second_receipt_is_unobserved(self):
        f=self.fixture();c=self.controls(f)
        c['phases']['overlap']['events'][3]['message']['content'].append({'type':'text','text':f['activation_marker']})
        self.assertEqual(self.classify(c,f),'activation_evidence_unobserved')

    def test_wrong_binding_version_in_either_scope_is_rejected(self):
        f=self.fixture()
        for scope in ['user','project']:
            c=self.controls(f)
            next(b for b in c['phases']['overlap']['bindings'] if b['scope']==scope)['version']='0.0.1'
            self.assertEqual(self.classify(c,f),'registration_mismatch')

    def test_two_resource_reads_are_counted_independently_of_one_skill_call(self):
        f=self.fixture();c=self.controls(f);row=c['phases']['overlap'];events=row['events']
        call=copy.deepcopy(events[4]);call['message']['content'][0]['id']='second-read'
        reply=copy.deepcopy(events[5]);reply['message']['content'][0]['tool_use_id']='second-read'
        events[-1:-1]=[call,reply]
        reason,evidence=q.inspect_claude_scope_phase(row,f,['user','project'])
        self.assertIsNone(reason);self.assertEqual(evidence['counts'],[1,1,1,1,2])
        self.assertEqual(self.classify(c,f),'control_failed')

    def test_stream_completeness_must_be_explicit_before_any_phase_is_counted(self):
        f=self.fixture()
        for phase in self.controls(f)['phases']:
            for value in [True,None,0,'missing']:
                with self.subTest(phase=phase,value=value):
                    c=self.controls(f)
                    if value=='missing':del c['phases'][phase]['malformed_stream']
                    else:c['phases'][phase]['malformed_stream']=value
                    expected='positive_control_failed' if phase.endswith('_positive') else 'activation_evidence_unobserved'
                    self.assertEqual(self.classify(c,f),expected)

    def test_each_calibration_requires_one_of_every_observed_component(self):
        f=self.fixture();namespace=f['plugin_name']+':'+f['skill_name']
        for phase in ['project_positive','user_positive']:
            for component in ['skills','slash_commands','Skill','body','Read']:
                for count in [0,2]:
                    with self.subTest(phase=phase,component=component,count=count):
                        c=self.controls(f);events=c['phases'][phase]['events']
                        if component in ['skills','slash_commands']:events[0][component]=[namespace]*count
                        elif component=='body':events[3]['message']['content']*=count
                        else:
                            index=1 if component=='Skill' else 4
                            if count==0:del events[index:index+2]
                            else:
                                call=copy.deepcopy(events[index]);reply=copy.deepcopy(events[index+1])
                                call['message']['content'][0]['id']='extra-'+component
                                reply['message']['content'][0]['tool_use_id']='extra-'+component
                                events[-1:-1]=[call,reply]
                        self.assertEqual(self.classify(c,f),'positive_control_failed')

    def test_empty_or_unrelated_catalogs_cannot_establish_any_candidate_phase(self):
        f=self.fixture()
        for phase in ['overlap','user_remaining','project_remaining']:
            for field in ['skills','slash_commands']:
                for catalog in [[],['another-package:probe']]:
                    with self.subTest(phase=phase,field=field,catalog=catalog):
                        c=self.controls(f);c['phases'][phase]['events'][0][field]=catalog
                        self.assertEqual(self.classify(c,f),'control_failed')

    def test_each_surviving_scope_requires_a_new_native_session(self):
        f=self.fixture()
        for phase in ['user_remaining','project_remaining']:
            c=self.controls(f);sid=c['phases']['overlap']['events'][0]['session_id']
            for event in c['phases'][phase]['events']:event['session_id']=sid
            self.assertEqual(self.classify(c,f),'session_identity_mismatch')

    def test_skill_and_read_tool_errors_reject_each_phase_independently(self):
        f=self.fixture()
        for phase in self.controls(f)['phases']:
            for index in [2,5]:
                with self.subTest(phase=phase,reply_index=index):
                    c=self.controls(f);c['phases'][phase]['events'][index]['message']['content'][0]['is_error']=True
                    expected='positive_control_failed' if phase.endswith('_positive') else 'control_failed'
                    self.assertEqual(self.classify(c,f),expected)

    def test_surviving_scope_duplicates_keep_the_duplicate_classification(self):
        f=self.fixture()
        for phase in ['user_remaining','project_remaining']:
            for component in ['skills','slash_commands','body']:
                with self.subTest(phase=phase,component=component):
                    c=self.controls(f);events=c['phases'][phase]['events']
                    if component=='body':events[3]['message']['content']*=2
                    else:events[0][component]*=2
                    self.assertEqual(self.classify(c,f),'duplicate_exposure_or_activation')
