"""Offline Q3 component fixtures and classification; never invoke Copilot."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import subprocess
from unittest import mock


class CopilotPluginComponentsTests(unittest.TestCase):
    def setUp(self):
        path = Path(__file__).parents[1] / 'qualify-native-instructions.py'
        spec = importlib.util.spec_from_file_location('q3', path)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)

    def test_standard_package_and_independent_scope_projects(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.copilot_plugin_component_fixture(Path(tmp) / 'fixture')
            self.assertEqual(f['pin'], '1.0.103')
            self.assertEqual(set(f['projects']), {'project', 'user'})
            self.assertNotEqual(*f['native_markers'].values())
            package = f['package']
            manifest = json.loads((package / 'plugin.json').read_text())
            self.assertEqual(manifest['$schema'], 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json')
            self.assertTrue((package / 'com.github.copilot/rules/plain.md').is_file())
            scoped = (package / 'com.github.copilot/rules/scoped.md').read_text()
            self.assertIn('applyTo: "matching/**"', scoped)
            self.assertIn(f['scoped_marker'], scoped)
            agent = (package / 'com.github.copilot/agents/probe.agent.md').read_text()
            self.assertIn(f['agent_name'], agent)
            self.assertIn('tools: [read]\n',agent)
            market=json.loads((f['marketplace']/'.claude-plugin/marketplace.json').read_text())
            self.assertEqual(market,{'name':f['market_name'],'owner':{'name':'Qualification Fixture'},'plugins':[{'name':f['plugin_name'],'source':'./packages/'+f['plugin_name']}]})
            for project in f['projects'].values():
                self.assertTrue((project / '.github/copilot-instructions.md').is_file())
                self.assertIn(f['catalog_positive_name'], (project / '.github/agents/catalog-positive.agent.md').read_text())
                self.assertEqual((project / 'agent-resource.txt').read_text().strip(), f['agent_marker'])
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name, ('AGENTS.md', 'AGENTS.override.md', 'CLAUDE.md'))

    def test_existing_fixture_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); sentinel = root / 'manual'; sentinel.write_text('keep')
            with self.assertRaises(ValueError):
                self.q.copilot_plugin_component_fixture(root)
            self.assertEqual(sentinel.read_text(), 'keep')

    def observations(self, scope="project"):

        f = {'pin': '1.0.103', 'native_markers': {'project': 'N', 'user': 'U'}, 'plain_marker': 'P', 'scoped_marker': 'S', 'agent_marker': 'A'}
        native = f['native_markers'][scope]
        def row(rules):return {'pin': '1.0.103', 'state': 'success', 'completed': True, 'response': {'native': native, 'rules': rules}}
        observations = {k: row(['P']) for k in ('installed', 'reinstalled', 'nonmatching')}
        observations.update({k: row([]) for k in ('native_positive', 'disabled', 'uninstalled')})
        observations['matching'] = row(['P', 'S'])
        for k in ('matching', 'nonmatching'):observations[k]['resource_read'] = True
        observations['agent'] = {'pin': '1.0.103', 'state': 'success', 'completed': True, 'catalog_present': True, 'delegated': True, 'child_resource_read': True, 'response': 'A'}
        return observations, f

    def assess(self, value, fixture):
        return self.q.classify_copilot_plugin_components(value, fixture, 'project')

    def test_every_installed_and_absent_lifecycle_result_is_required(self):
        value, f = self.observations()
        self.assertEqual(self.assess(value, f), 'observed')
        for stage in ('installed', 'reinstalled', 'matching', 'nonmatching', 'disabled', 'uninstalled'):
            value, f = self.observations(); value[stage]['response']['rules'] = ['WRONG']
            self.assertNotEqual(self.assess(value, f), 'observed')
            value, f = self.observations(); del value[stage]
            self.assertNotEqual(self.assess(value, f), 'observed')

    def test_failed_positive_never_becomes_absent_or_unsupported_read(self):
        value, f = self.observations(); value['native_positive']['state'] = 'authentication_failure'
        self.assertEqual(self.assess(value, f), 'positive_control_failed')
        value, f = self.observations(); value['native_positive']['response'] = {'native': 'WRONG', 'rules': []}
        self.assertEqual(self.assess(value, f), 'positive_control_failed')

    def test_scope_requires_matching_and_nonmatching_reads_and_agent_is_real_child(self):
        for stage in ('matching', 'nonmatching'):
            value, f = self.observations(); value[stage]['resource_read'] = False
            self.assertNotEqual(self.assess(value, f), 'observed')
        for key, replacement in [('delegated', False), ('catalog_present', False), ('child_resource_read', False), ('response', 'WRONG'), ('completed', False)]:
            value, f = self.observations(); value['agent'][key] = replacement
            self.assertNotEqual(self.assess(value, f), 'observed')
        for stage in ('installed', 'disabled', 'uninstalled', 'reinstalled', 'matching', 'nonmatching'):
            value, f = self.observations(); value[stage]['state'] = 'timeout'
            self.assertNotEqual(self.assess(value, f), 'observed')

    def test_every_control_requires_the_same_native_pin_and_explicit_completion(self):
        for stage in ('native_positive', 'installed', 'matching', 'nonmatching', 'agent', 'disabled', 'uninstalled', 'reinstalled'):
            for key, replacement in (('pin', 'different'), ('completed', None), ('completed', 1)):
                value, f = self.observations(); value[stage][key] = replacement
                self.assertNotEqual(self.assess(value, f), 'observed')

    def test_fixture_role_markers_are_exact_and_child_answer_is_only_in_resources(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.copilot_plugin_component_fixture(Path(tmp) / 'fixture')
            plain = (f['package'] / 'com.github.copilot/rules/plain.md').read_text()
            self.assertEqual(plain, '---\nname: ' + f['plugin_name'] + '-plain\ndescription: Global component qualification rule.\napplyTo: "**"\n---\nFor the component qualification question include ' + f['plain_marker'] + ' in the rules array.\n')
            self.assertNotIn(f['scoped_marker'], plain)
            scoped = (f['package'] / 'com.github.copilot/rules/scoped.md').read_text()
            self.assertNotIn(f['plain_marker'], scoped)
            self.assertIn('name: ' + f['plugin_name'] + '-scoped', scoped)
            self.assertIn('description: Scoped component qualification rule.', scoped)
            for scope, project in f['projects'].items():
                text = (project / '.github/copilot-instructions.md').read_text()
                self.assertIn(f['native_markers'][scope], text)
                for marker in [f['plain_marker'], f['scoped_marker'], *[v for k,v in f['native_markers'].items() if k != scope]]:
                    self.assertNotIn(marker, text)
            hits = [p for p in f['root'].rglob('*') if p.is_file() and '.git' not in p.parts and f['agent_marker'] in p.read_text()]
            self.assertEqual(set(hits), {p / 'agent-resource.txt' for p in f['projects'].values()})
            env = self.q.isolated_git_environment()
            commits = []
            for project in f['projects'].values():
                commits.append(subprocess.check_output(['git','-C',str(project),'rev-parse','HEAD'],env=env))
                self.assertEqual(subprocess.check_output(['git','-C',str(project),'show','-s','--format=%P','HEAD'],env=env).strip(),b'')
                self.assertEqual(subprocess.check_output(['git','-C',str(project),'ls-tree','-r','--name-only','HEAD'],env=env),b'')
            self.assertNotEqual(*commits)

    def test_both_scopes_have_independent_exact_response_envelopes(self):
        for scope, other in [('project','user'),('user','project')]:
            value, f = self.observations(scope)
            self.assertEqual(self.q.classify_copilot_plugin_components(value, f, scope), 'observed')
            self.assertNotEqual(self.q.classify_copilot_plugin_components(value, f, other), 'observed')
            for stage in ('installed','matching','nonmatching','disabled','uninstalled','reinstalled'):
                for change in ('wrong_native','missing_native','extra_field'):
                    value, f = self.observations(scope)
                    if change == 'wrong_native':value[stage]['response']['native'] = 'WRONG'
                    elif change == 'missing_native':del value[stage]['response']['native']
                    else:value[stage]['response']['extra'] = 'unexpected'
                    self.assertNotEqual(self.q.classify_copilot_plugin_components(value, f, scope), 'observed')
            value, f = self.observations(scope);value['native_positive']['response']['rules'] = ['P']
            self.assertNotEqual(self.q.classify_copilot_plugin_components(value, f, scope), 'observed')

    def test_child_and_read_evidence_is_literal_true_and_child_state_is_success(self):
        for phase, fields in [('matching',('resource_read',)),('nonmatching',('resource_read',)),('agent',('catalog_present','delegated','child_resource_read'))]:
            for field in fields:
                for changed in (False,None,1,'yes'):
                    value, f = self.observations();value[phase][field] = changed
                    self.assertNotEqual(self.assess(value,f), 'observed')
                value, f = self.observations();del value[phase][field]
                self.assertNotEqual(self.assess(value,f), 'observed')
        for state in ('failure','timeout','authentication_failure'):
            value, f = self.observations();value['agent']['state'] = state
            self.assertNotEqual(self.assess(value,f), 'observed')

    def test_neutral_root_guard_precedes_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for relative in ('.claude/x','.github/x','.codex/x','.opencode/x','.copilot/instructions/x','.config/opencode/x','.agents/skills/x','.agents/plugins/x'):
                before=set(root.rglob('*'))
                with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(root/relative)
                self.assertEqual(set(root.rglob('*')),before)
            with mock.patch.dict(self.q.os.environ,{'COPILOT_HOME':str(root/'relocated-client-home')}):
                before=set(root.rglob('*'))
                with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(root/'relocated-client-home/instructions/fixture')
                self.assertEqual(set(root.rglob('*')),before)
            with mock.patch.object(self.q,'__file__',str(root.resolve()/'repository/scripts/harness.py')):
                with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(root/'repository/fixture')

    def test_plugin_catalog_selection_uses_namespace_identity_and_display_name(self):
        options=[{'id':'agent','options':[{'value':'plugin:probe','name':'display-child'}]}]
        self.assertEqual(self.q.copilot_plugin_agent_selection(options,'plugin','display-child'),'plugin:probe')
        self.assertIsNone(self.q.copilot_plugin_agent_selection(options,'other','display-child'))
        self.assertIsNone(self.q.copilot_plugin_agent_selection(options,'plugin','other-child'))
        options[0]['options'].append({'value':'plugin:other','name':'display-child'})
        self.assertIsNone(self.q.copilot_plugin_agent_selection(options,'plugin','display-child'))
        non_agent={'id':'model','options':[{'value':'plugin:foreign','name':'display-child'}]}
        self.assertIsNone(self.q.copilot_plugin_agent_selection([non_agent],'plugin','display-child'))
        agent={'id':'agent','options':[{'value':'plugin:probe','name':'display-child'}]}
        self.assertEqual(self.q.copilot_plugin_agent_selection([non_agent,agent],'plugin','display-child'),'plugin:probe')

    def test_delegation_policy_bounds_agent_model_and_resource(self):
        policy = lambda tool,args: self.q.copilot_delegation_policy(tool,args,'plugin:probe','preferred',Path('/fixture/agent-resource.txt'))
        self.assertEqual(policy('task',{'agent_type':'plugin:probe'}),'deny')
        self.assertEqual(policy('task',{'agent_type':'plugin:probe','model':'preferred'}),'allow')
        for args in ({'agent_type':'general-purpose','model':'preferred'},{'agent_type':'plugin:probe','model':'other'},{'agent_type':'plugin:probe','model':None},{'agent_type':'plugin:probe','model':'preferred','resume':'old'},{'agent_type':'plugin:probe','model':'preferred','mode':'background'},{},{'agent_type':None}):
            self.assertEqual(policy('task',args),'deny')
        for continuation in ('resume','agent_id','session_id'):
            self.assertEqual(policy('task',{'agent_type':'plugin:probe','model':'preferred',continuation:'old'}),'deny')
        for mode in ('sync','foreground'):
            self.assertEqual(policy('task',{'agent_type':'plugin:probe','model':'preferred','mode':mode}),'allow')
        for tool in ('view','read','read_file'):
            self.assertEqual(policy(tool,{'file_path':'/fixture/agent-resource.txt'}),'allow')
        self.assertEqual(policy('view',{'path':'/outside/agent-resource.txt'}),'deny')
        self.assertEqual(policy('view',{'path':'agent-resource.txt'}),'allow')
        self.assertEqual(policy('view',{'path':'/fixture/agent-resource.txt'}),'allow')
        for tool,args in [('view',{'path':'../outside.txt'}),('view',{}),('view',{'path':'agent-resource.txt','file_path':'/outside'}),('bash',{'command':'anything'}),('create',{'path':'agent-resource.txt'}),('unknown',{}),('task',None)]:
            self.assertEqual(policy(tool,args),'deny')

    def test_native_instruction_suffix_variant_is_explicit_and_bounded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            f = self.q.copilot_plugin_component_fixture(root/'variant', rule_suffix='.instructions.md')
            rules = f['package']/'com.github.copilot/rules'
            self.assertEqual({p.name for p in rules.iterdir()}, {'plain.instructions.md','scoped.instructions.md'})
            self.assertIn(f['plain_marker'], (rules/'plain.instructions.md').read_text())
            self.assertIn(f['scoped_marker'], (rules/'scoped.instructions.md').read_text())
            for stem,scope in (('plain','**'),('scoped','matching/**')):
                text=(rules/(stem+'.instructions.md')).read_text()
                self.assertTrue(text.startswith('---\nname: '+f['plugin_name']+'-'+stem+'\ndescription: '))
                self.assertIn('\napplyTo: \"'+scope+'\"\n---\n',text)
            self.assertEqual(f['rule_suffix'], '.instructions.md')
            with self.assertRaises(ValueError):
                self.q.copilot_plugin_component_fixture(root/'bad', rule_suffix='/../../outside')
            self.assertFalse((root/'bad').exists())

    def test_exact_child_marker_rejects_leading_and_trailing_whitespace(self):
        for response in (' A','A ','\nA\n','\tA'):
            value,f=self.observations();value['agent']['response']=response
            self.assertEqual(self.assess(value,f),'agent_control_failed')

    def test_symlinked_default_native_home_is_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);home=root/'home';home.mkdir()
            target=root/'native-profile-storage';target.mkdir()
            (home/'.copilot').symlink_to(target,target_is_directory=True)
            with mock.patch.dict(self.q.os.environ,{'HOME':str(home),'COPILOT_HOME':''}):
                for candidate in (home/'.copilot/instructions/fixture',target/'instructions/physical-fixture'):
                    before=set(root.rglob('*'))
                    with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(candidate)
                    self.assertEqual(set(root.rglob('*')),before)

    def test_tilde_client_home_is_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);home=root/'home';home.mkdir()
            before=set(root.rglob('*'))
            with mock.patch.dict(self.q.os.environ,{'HOME':str(home),'COPILOT_HOME':'~/relocated-client-home'}):
                with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(home/'relocated-client-home/instructions/fixture')
            self.assertEqual(set(root.rglob('*')),before)

    def test_neutral_root_rejects_aliases_other_client_homes_and_ancestor_guidance(self):
        cases=[('/offline/.CLAUDE/fixture',{}),('/offline/custom-codex/fixture',{'CODEX_HOME':'/offline/custom-codex'}),('/offline/team/fixture',{})]
        for candidate,environment in cases:
            with self.subTest(candidate=candidate), mock.patch.dict(self.q.os.environ,environment,clear=True), mock.patch.object(self.q.os.path,'lexists',return_value=False), mock.patch.object(Path,'resolve',lambda p:p), mock.patch.object(Path,'home',return_value=Path('/offline/user')), mock.patch.object(Path,'exists',lambda p:str(p)=='/offline/team/AGENTS.md'), mock.patch.object(Path,'mkdir',side_effect=AssertionError('Fixture must reject before writes')):
                with self.assertRaises(ValueError):self.q.copilot_plugin_component_fixture(Path(candidate))
