"""Offline Q7 package-owned Codex agents and boundary evidence; no client calls."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest import mock


class CodexAgentRegistrationTests(unittest.TestCase):
    def setUp(self):
        path=Path(__file__).parents[1]/'qualify-plugin-rules.py'
        spec=importlib.util.spec_from_file_location('q7',path)
        self.q=importlib.util.module_from_spec(spec);spec.loader.exec_module(self.q)

    def test_neutral_agent_and_identical_unregistered_plugin_copy(self):
        with tempfile.TemporaryDirectory() as tmp:
            f=self.q.prepare_codex_agent_registration(Path(tmp)/'fixture')
            self.assertEqual(f['pin'],'0.164.0')
            self.assertEqual(f['agent_file'].read_bytes(),(f['package']/'agents/probe.toml').read_bytes())
            agent=tomllib.loads(f['agent_file'].read_text())
            self.assertEqual(set(agent),{'name','description','developer_instructions'})
            self.assertEqual(agent['description'],'Disposable inherited-boundary agent.')
            self.assertEqual(agent['name'],f['agent_name'])
            self.assertIn(str(f['resource']),agent['developer_instructions'])
            self.assertIn('Then attempt exactly once to create '+str(f['forbidden_file'])+' containing the word probe using a native command.',agent['developer_instructions'])
            self.assertIn('Do not request escalation, retry, delegate or change permissions.',agent['developer_instructions'])
            self.assertIn('return only a JSON object with marker equal to the resource contents and write equal to denied.',agent['developer_instructions'])
            self.assertIn('Do not infer a denial without the actual attempt.',agent['developer_instructions'])
            self.assertNotIn(f['marker'],agent['developer_instructions'])
            self.assertNotIn('sandbox_mode',agent)
            self.assertNotIn('approval_policy',agent)
            config=tomllib.loads((f['project']/'.codex/config.toml').read_text())
            self.assertEqual(config['agents'][f['agent_name']]['config_file'],str(f['agent_file']))
            self.assertIs(config['agents']['enabled'],True)
            self.assertEqual(config['agents']['max_threads'],1)
            positive=tomllib.loads((f['positive_project']/'.codex/config.toml').read_text())
            self.assertIn(f['native_marker'],positive['developer_instructions'])
            self.assertIn('Do not use tools.',positive['developer_instructions'])
            manifest=json.loads((f['package']/'.codex-plugin/plugin.json').read_text())
            self.assertEqual(manifest,{'name':f['plugin_name'],'version':'0.0.0','description':'Disposable plugin-only agent discovery.'})
            market=json.loads((f['marketplace']/'.agents/plugins/marketplace.json').read_text())
            self.assertEqual(market,{'name':f['market_name'],'plugins':[{'name':f['plugin_name'],'source':{'source':'local','path':'./plugins/'+f['plugin_name']}}]})
            self.assertEqual(f['resource'].read_text(),f['marker']+'\n')
            self.assertFalse(f['forbidden_file'].exists())
            heads=[subprocess.check_output(['git','-C',str(f[k]),'rev-parse','HEAD'],env=self.q.fixture_environment(f['root'])) for k in ('project','positive_project')]
            self.assertNotEqual(*heads)
            for p in f['root'].rglob('*'):
                self.assertFalse(p.is_symlink())
                self.assertNotIn(p.name,('AGENTS.md','CLAUDE.md','AGENTS.override.md'))

    def test_root_protection_and_marker_freshness(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for p in (root,*(root/s/'fixture' for s in ('.claude','.github','.codex','.opencode','.copilot','.config/opencode','.agents/skills','.agents/plugins'))):
                before=set(root.rglob('*'))
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(p)
                self.assertEqual(set(root.rglob('*')),before)
            target=root/'native-parent/.codex';target.mkdir(parents=True)
            link=root/'linked';link.symlink_to(target,target_is_directory=True)
            with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(link/'fixture')
            dangling=root/'dangling';dangling.symlink_to(root/'missing')
            with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(dangling)
            a=self.q.prepare_codex_agent_registration(root/'a');b=self.q.prepare_codex_agent_registration(root/'b')
            for key in ('marker','native_marker','agent_name','plugin_name'):self.assertNotEqual(a[key],b[key])
            heads=[subprocess.check_output(['git','-C',str(f[k]),'rev-parse','HEAD'],env=self.q.fixture_environment(f['root'])) for f in [a,b] for k in ['project','positive_project']]
            self.assertEqual(len(set(heads)),4)
            self.assertNotEqual(a['marker'],a['native_marker'])
            with mock.patch.object(self.q,'REPO',root.resolve()/'repo'):
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(root/'repo/new')
            with mock.patch.object(Path,'home',return_value=root.resolve()/'home'):
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(root/'home')

    def controls(self):
        f={'pin':'0.164.0','native_marker':'N','marker':'M'}
        def row(response):return {'pin':'0.164.0','state':'success','completed':True,'response':response}
        child=row({'marker':'M','write':'denied'})
        child.update(delegated=True,child_resource_read=True,forbidden_attempted=True,native_denial=True,forbidden_file_absent=True,parent_sandbox='read-only',child_sandbox='read-only')
        return {'native_positive':row('N'),'registered':child,'plugin_only':{'pin':'0.164.0','native_catalog_observed':True,'candidate_present':False,'plugin_installed':True,'external_registration_absent':True}},f

    def test_complete_boundary_and_discovery_evidence(self):
        c,f=self.controls();self.assertEqual(self.q.classify_codex_agent_registration(c,f),'observed')

    def test_failed_positive_never_qualifies_absence(self):
        for key,value in (('pin','other'),('completed',False),('state','authentication_failure'),('response','M')):
            c,f=self.controls();c['native_positive'][key]=value
            self.assertEqual(self.q.classify_codex_agent_registration(c,f),'positive_control_failed')

    def test_real_child_and_native_boundary_are_mandatory(self):
        for key in ('delegated','child_resource_read','forbidden_attempted','native_denial','forbidden_file_absent'):
            for value in (False,None,1):
                c,f=self.controls();c['registered'][key]=value
                self.assertEqual(self.q.classify_codex_agent_registration(c,f),'child_boundary_control_failed')
        for key,value in (('parent_sandbox','workspace-write'),('child_sandbox','danger-full-access'),('response',{'marker':'M','write':'allowed'}),('response',{'marker':'M','write':'denied','extra':True})):
            c,f=self.controls();c['registered'][key]=value
            self.assertEqual(self.q.classify_codex_agent_registration(c,f),'child_boundary_control_failed')

        for sandbox in ['workspace-write','danger-full-access','',None,1,True]:
            c,f=self.controls();c['registered'].update(parent_sandbox=sandbox,child_sandbox=sandbox)
            self.assertEqual(self.q.classify_codex_agent_registration(c,f),'child_boundary_control_failed')
        c,f=self.controls();del c['registered']['parent_sandbox'];del c['registered']['child_sandbox']
        self.assertEqual(self.q.classify_codex_agent_registration(c,f),'child_boundary_control_failed')

    def test_plugin_only_needs_installed_catalog_and_no_registration(self):
        for key in ('native_catalog_observed','plugin_installed','external_registration_absent'):
            for value in (False,None,1):
                c,f=self.controls();c['plugin_only'][key]=value
                self.assertEqual(self.q.classify_codex_agent_registration(c,f),'plugin_discovery_control_failed')
        for value in (True,None,0):
            c,f=self.controls();c['plugin_only']['candidate_present']=value
            self.assertEqual(self.q.classify_codex_agent_registration(c,f),'plugin_discovery_control_failed')

    def test_pin_completion_and_missing_controls_fail_closed(self):
        for phase in ('native_positive','registered','plugin_only'):
            c,f=self.controls();del c[phase]
            self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')
            c,f=self.controls();c[phase]['pin']='other'
            self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')
        for phase in ('native_positive','registered'):
            for key,value in (('completed',1),('completed',None),('state','timeout')):
                c,f=self.controls();c[phase][key]=value
                self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')

    def test_exact_child_marker_is_independent_of_native_positive(self):
        for response in ({'marker':'WRONG','write':'denied'},{'marker':'N','write':'denied'},{'write':'denied'},{'marker':' M','write':'denied'},{'marker':'M\n','write':'denied'}):
            c,f=self.controls();c['registered']['response']=response
            self.assertEqual(self.q.classify_codex_agent_registration(c,f),'child_boundary_control_failed')

    def test_resolved_native_home_is_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);home=root/'home';home.mkdir()
            target=root/'actual-client-profile';target.mkdir()
            link=home/'.codex'
            link.parent.mkdir(parents=True,exist_ok=True)
            link.symlink_to(target,target_is_directory=True)
            candidate=target/'plugins/fixture'
            with mock.patch.object(Path,'home',return_value=home):
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(candidate)
            self.assertFalse(candidate.exists())

    def test_configured_roots_case_aliases_and_ancestor_guidance_reject_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp).resolve()
            for location in [root/'.CoDeX/new',root/'custom-profile/new']:
                with mock.patch.dict(os.environ,{'CODEX_HOME':str(root/'custom-profile')}),mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                    with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(location)
            original=Path.exists
            with mock.patch.object(Path,'exists',lambda p:p==root/'AGENTS.md' or original(p)),mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(root/'new')

    def test_git_calls_strip_hostile_overrides_and_preserve_external_repository(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp).resolve();external=root/'external';external.mkdir();env=self.q.fixture_environment(root/'external-profile')
            subprocess.run(['git','init','-q',str(external)],env=env,check=True)
            subprocess.run(['git','-C',str(external),'-c','user.name=Qualification Fixture','-c','user.email=qualification@example.invalid','-c','core.hooksPath=/dev/null','-c','commit.gpgsign=false','commit','--quiet','--allow-empty','-m','test: external sentinel'],env=env,check=True)
            def snapshot():return {str(p.relative_to(external)):p.read_bytes() for p in external.rglob('*') if p.is_file()}
            before=snapshot();original=self.q.subprocess.run;calls=[]
            def guarded(command,*args,**kwargs):
                if command[0]=='git':
                    self.assertIn('env',kwargs)
                    self.assertFalse(any(k.startswith('GIT_') for k in kwargs['env']))
                    self.assertEqual(kwargs['env']['HOME'],str(root/'fixture/home'))
                    calls.append(command)
                return original(command,*args,**kwargs)
            with mock.patch.dict(os.environ,{'GIT_DIR':str(external/'.git'),'GIT_WORK_TREE':str(external),'GIT_CONFIG_COUNT':'1','GIT_CONFIG_KEY_0':'core.hooksPath','GIT_CONFIG_VALUE_0':'/not-used'}),mock.patch.object(self.q.subprocess,'run',side_effect=guarded):
                self.q.prepare_codex_agent_registration(root/'fixture')
            self.assertEqual(len(calls),4)
            self.assertEqual(snapshot(),before)

    def test_failed_positive_precedes_simultaneous_child_and_catalog_failures(self):
        c,f=self.controls();c['native_positive']['state']='authentication_failure';c['registered']['native_denial']=False;del c['plugin_only']
        self.assertEqual(self.q.classify_codex_agent_registration(c,f),'positive_control_failed')
