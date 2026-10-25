"""Offline Q8 native plugin skill update fixtures and classifications."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


class CodexSkillUpdatesTests(unittest.TestCase):
    def setUp(self):
        p=Path(__file__).parents[1]/'qualify-plugin-rules.py';s=importlib.util.spec_from_file_location('q8',p);self.q=importlib.util.module_from_spec(s);s.loader.exec_module(self.q)

    def test_package_has_one_native_skill_and_private_resource_response(self):
        with tempfile.TemporaryDirectory() as tmp:
            f=self.q.prepare_codex_skill_updates(Path(tmp)/'fixture')
            self.assertEqual(f['pin'],'0.164.0')
            manifest=json.loads((f['package']/'.codex-plugin/plugin.json').read_text())
            self.assertEqual(manifest,{'name':f['plugin_name'],'version':'0.0.0','description':'Disposable skill update qualification.','skills':'./skills/'})
            entries=list(f['package'].rglob('SKILL.md'));self.assertEqual(entries,[f['skill_file']])
            body=f['skill_file'].read_text()
            self.assertTrue(body.startswith('---\nname: '+f['skill_name']+'\n'))
            self.assertIn('Read references/procedure.md relative to this skill',body)
            self.assertIn(f['fallback'],body)
            for m in (f['initial_marker'],f['edited_marker']):self.assertNotIn(m,body)
            expected_resource=f['skill_file'].parent/'references/procedure.md'
            self.assertEqual(f['resource'],expected_resource)
            self.assertEqual(expected_resource.read_text(),'Return exactly '+f['initial_marker']+' and nothing else.\n')
            self.assertNotEqual(f['initial_marker'],f['edited_marker'])
            market=json.loads((f['marketplace']/'.agents/plugins/marketplace.json').read_text())
            self.assertEqual(market['name'],f['market_name'])
            self.assertEqual(market['plugins'],[{'name':f['plugin_name'],'source':{'source':'local','path':'./plugins/'+f['plugin_name']}}])
            heads=[subprocess.check_output(['git','-C',str(f[k]),'rev-parse','HEAD'],env=self.q.fixture_environment(f['root'])) for k in ('project','unrelated_project')];self.assertNotEqual(*heads)
            for project in [f['project'],f['unrelated_project']]:
                env=self.q.fixture_environment(f['root'])
                self.assertEqual(subprocess.check_output(['git','-C',str(project),'rev-list','--count','HEAD'],env=env).strip(),b'1')
                self.assertEqual(subprocess.check_output(['git','-C',str(project),'ls-tree','--name-only','HEAD'],env=env),b'')
            for p in f['root'].rglob('*'):
                if p.is_file() and '.git' not in p.relative_to(f['root']).parts and p!=f['resource']:
                    for marker in [f['initial_marker'],f['edited_marker']]:self.assertNotIn(marker,p.read_text())
                self.assertFalse(p.is_symlink())
                self.assertNotIn(p.name,('CLAUDE.md','AGENTS.md','AGENTS.override.md'))
            self.assertEqual(list(f['package'].rglob('hooks.json')),[])

    def test_root_exclusions_and_fresh_values(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for p in (root,*(root/s/'fixture' for s in ('.claude','.github','.codex','.opencode','.copilot','.config/opencode','.agents/skills','.agents/plugins'))):
                before=set(root.rglob('*'))
                with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(p)
                self.assertEqual(set(root.rglob('*')),before)
            a=self.q.prepare_codex_skill_updates(root/'a');b=self.q.prepare_codex_skill_updates(root/'b')
            for k in ('initial_marker','edited_marker','fallback','plugin_name','skill_name'):self.assertNotEqual(a[k],b[k])
            heads=[subprocess.check_output(['git','-C',str(f[k]),'rev-parse','HEAD'],env=self.q.fixture_environment(f['root'])) for f in [a,b] for k in ['project','unrelated_project']]
            self.assertEqual(len(set(heads)),4)
            with mock.patch.object(self.q,'REPO',root.resolve()/'repo'):
                with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(root/'repo/new')
            with mock.patch.object(Path,'home',return_value=root.resolve()/'home'):
                with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(root/'home')

    def controls(self):
        f={'pin':'0.164.0','initial_marker':'A','edited_marker':'B'}
        def row(response):return dict(pin='0.164.0',state='success',completed=True,response=response,native_expansion=True,resource_read=True,native_identity='plugin:probe',load_path='/native/cache/skills/probe/SKILL.md')
        return {'installed':row('A'),'edited_without_reinstall':row('A'),'reinstalled':row('B')},f

    def test_edit_visibility_recorded_either_way(self):
        for marker,result in (('A','edits_require_refresh'),('B','edits_visible_without_refresh')):
            c,f=self.controls();c['edited_without_reinstall']['response']=marker
            self.assertEqual(self.q.classify_codex_skill_updates(c,f),result)

    def test_failed_installed_positive_is_never_cache_visibility(self):
        for key,value in (('state','authentication_failure'),('completed',False),('response','B'),('native_expansion',False),('resource_read',False)):
            c,f=self.controls();c['installed'][key]=value
            self.assertEqual(self.q.classify_codex_skill_updates(c,f),'positive_control_failed')

    def test_each_stage_requires_pin_completion_expansion_and_resource_read(self):
        for phase in ('installed','edited_without_reinstall','reinstalled'):
            for key,value in (('pin','other'),('state','timeout'),('completed',1),('completed',None),('native_expansion',False),('native_expansion',1),('resource_read',False),('resource_read',1)):
                c,f=self.controls();c[phase][key]=value
                self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))
            c,f=self.controls();del c[phase]
            self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))

    def test_identity_and_load_path_are_real_and_correlated(self):
        for phase in ('installed','edited_without_reinstall','reinstalled'):
            for key in ('native_identity','load_path'):
                for invalid in ('',None,1,[]):
                    c,f=self.controls();c[phase][key]=invalid
                    self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))
        for phase in ('edited_without_reinstall','reinstalled'):
            c,f=self.controls();c[phase]['native_identity']='other:probe'
            self.assertEqual(self.q.classify_codex_skill_updates(c,f),'identity_mismatch')

    def test_unexpected_edit_response_or_old_after_refresh_cannot_pass(self):
        for phase,value in (('edited_without_reinstall','unknown'),('reinstalled','A'),('reinstalled','B\n')):
            c,f=self.controls();c[phase]['response']=value
            self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))

        for phase,marker in (('installed','A'),('edited_without_reinstall','A'),('reinstalled','B')):
            for response in (' '+marker,marker+' ',marker+'\n'):
                c,f=self.controls();c[phase]['response']=response
                self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))

    def test_missing_required_evidence_fails_closed(self):
        for phase in ('installed','edited_without_reinstall','reinstalled'):
            for key in ('pin','state','completed','response','native_expansion','resource_read','native_identity','load_path'):
                c,f=self.controls();del c[phase][key]
                self.assertNotIn(self.q.classify_codex_skill_updates(c,f),('edits_require_refresh','edits_visible_without_refresh'))

    def test_resolved_native_home_is_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);home=root/'home';home.mkdir()
            target=root/'actual-client-profile';target.mkdir()
            link=home/'.codex'
            link.parent.mkdir(parents=True,exist_ok=True)
            link.symlink_to(target,target_is_directory=True)
            candidate=target/'plugins/fixture'
            with mock.patch.object(Path,'home',return_value=home):
                with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(candidate)
            self.assertFalse(candidate.exists())

    def test_configured_roots_case_aliases_and_ancestor_guidance_reject_before_writes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp).resolve()
            for location in [root/'.CoDeX/new',root/'custom-profile/new']:
                with mock.patch.dict(os.environ,{'CODEX_HOME':str(root/'custom-profile')}),mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                    with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(location)
            original=Path.exists
            with mock.patch.object(Path,'exists',lambda p:p==root/'AGENTS.md' or original(p)),mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                with self.assertRaises(ValueError):self.q.prepare_codex_skill_updates(root/'new')

    def test_failed_positive_precedes_simultaneous_lifecycle_failures(self):
        for key,value in [('state','authentication_failure'),('response','B')]:
            c,f=self.controls();c['installed'][key]=value;del c['reinstalled'];c['edited_without_reinstall']['completed']=False
            self.assertEqual(self.q.classify_codex_skill_updates(c,f),'positive_control_failed')

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
                self.q.prepare_codex_skill_updates(root/'fixture')
            self.assertEqual(len(calls),4)
            self.assertEqual(snapshot(),before)
