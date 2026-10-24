"""Q5 explicit-path lifecycle controls, without native or model requests."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC=importlib.util.spec_from_file_location('q5_explicit',Path(__file__).parents[1]/'qualify-native-instructions.py')
q=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(q)

class OpenCodeExplicitPathTests(unittest.TestCase):
    def test_fixture_keeps_native_rules_outside_discovery_until_registered(self):
        self.assertTrue(callable(getattr(q,'opencode_explicit_path_fixtures',None)),'Missing explicit-path fixture builder')
        with tempfile.TemporaryDirectory() as temporary:
            f=q.opencode_explicit_path_fixtures(Path(temporary)/'fixture')
            self.assertEqual(f['pin'],'2.1.4')
            self.assertEqual(len(f['rule_files']),2)
            markers=[f['positive_marker'],f['fallback'],*f['initial_markers'],*f['updated_markers']]
            self.assertEqual(len(set(markers)),6)
            for project in [f['project'],f['positive_project'],f['outside_project']]:
                config=json.loads((project/'opencode.json').read_text())
                self.assertEqual(config['permissions'],[{'action':'*','resource':'*','effect':'deny'}])
                if project==f['positive_project']:
                    self.assertEqual(config['instructions'],[str(project/'native-positive.md')])
                    self.assertIn(f['positive_marker'],(project/'native-positive.md').read_text())
                else:self.assertNotIn('instructions',config)
            for index,path in enumerate(f['rule_files']):
                self.assertTrue(path.is_relative_to(f['root']/'delivery'))
                self.assertIn(f['initial_markers'][index],path.read_text())
                self.assertNotIn(f['updated_markers'][index],path.read_text())
                self.assertNotIn(f['fallback'],path.read_text())
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name,('AGENTS.md','AGENTS.override.md','CLAUDE.md','CLAUDE.local.md'))
            self.assertFalse((f['root']/'home').exists())
            env={key:value for key,value in os.environ.items() if not key.startswith('GIT_')};env.update(HOME=str(f['root']/'home'),USERPROFILE=str(f['root']/'home'))
            heads=[subprocess.check_output(['git','-C',str(p),'rev-parse','HEAD'],env=env) for p in [f['project'],f['positive_project'],f['outside_project']]]
            self.assertEqual(len(set(heads)),3)
            other=q.opencode_explicit_path_fixtures(Path(temporary)/'second')
            other_markers=[other['positive_marker'],other['fallback'],*other['initial_markers'],*other['updated_markers']]
            self.assertTrue(set(markers).isdisjoint(other_markers))

    def test_existing_client_roots_and_ancestor_guidance_are_rejected_without_writes(self):
        self.assertTrue(callable(getattr(q,'opencode_explicit_path_fixtures',None)),'Missing explicit-path fixture builder')
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve()
            with self.assertRaises(ValueError):q.opencode_explicit_path_fixtures(root)
            for component in ['.claude','.codex','.github','.opencode','.agents/skills']:
                with self.assertRaises(ValueError):q.opencode_explicit_path_fixtures(root/component/'new')
            original=Path.exists;guide=root/'AGENTS.md'
            with mock.patch.object(Path,'exists',lambda p:p==guide or original(p)),mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                with self.assertRaises(ValueError):q.opencode_explicit_path_fixtures(root/'new')

    def controls(self):
        f={'pin':'2.1.4','positive_marker':'N','fallback':'F','initial_markers':['A','B'],'updated_markers':['U','V'],'rule_files':[Path('/managed/a.md'),Path('/managed/b.md')],'project':Path('/project'),'positive_project':Path('/positive'),'outside_project':Path('/outside')}
        c={}
        for label in ['positive','unregistered','installed','resume','updated','removed','reinstalled','outside']:
            expected='N' if label=='positive' else 'F' if label in ['unregistered','removed','outside'] else {'rules':['A','B'] if label in ['installed','resume'] else ['U','V']}
            c[label]={'pin':'2.1.4','state':'success','completed':True,'response':expected,'tools_observed':False,'session':label,'cwd':'/positive' if label=='positive' else '/outside' if label=='outside' else '/project','native_instruction_paths':[] if label in ['positive','unregistered','removed','outside'] else ['/managed/a.md','/managed/b.md'],'native_body_counts':{} if label in ['positive','unregistered','removed','outside'] else {'a':1,'b':1}}
        c['resume']['session']='installed'
        return c,f

    def classify(self,c,f):
        self.assertTrue(callable(getattr(q,'classify_opencode_explicit_paths',None)),'Missing explicit-path classifier')
        return q.classify_opencode_explicit_paths(c,f)

    def test_positive_failure_precedes_every_candidate_result(self):
        c,f=self.controls();c['positive']['state']='failure';del c['installed']
        self.assertEqual(self.classify(c,f),'positive_control_failed')
        for key,value in [('response','F'),('completed',1),('tools_observed',True),('pin','wrong')]:
            c,f=self.controls();c['positive'][key]=value
            self.assertEqual(self.classify(c,f),'positive_control_failed')

    def test_each_lifecycle_response_has_exact_marker_identity_and_multiplicity(self):
        c,f=self.controls();self.assertEqual(self.classify(c,f),'observed')
        for stage in ['installed','resume','updated','reinstalled']:
            for response in [{'rules':['X','Y']},{'rules':['A','A','B']},{'rules':['U']},'A B']:
                c,f=self.controls();c[stage]['response']=response
                self.assertEqual(self.classify(c,f),'rule_control_failed')
            c,f=self.controls();c[stage]['response']['rules'].reverse()
            self.assertEqual(self.classify(c,f),'observed')
        for stage in ['unregistered','removed','outside']:
            c,f=self.controls();c[stage]['response']='A'
            self.assertEqual(self.classify(c,f),'absence_control_failed')

    def test_response_alone_does_not_establish_native_file_identity_or_single_delivery(self):
        for stage in ['installed','resume','updated','reinstalled']:
            for key in ['native_instruction_paths','native_body_counts']:
                c,f=self.controls();del c[stage][key]
                self.assertEqual(self.classify(c,f),'native_identity_unobserved')
            for value in [{'a':2,'b':1},{'a':True,'b':1},{'a':1},{'a':1,'b':1,'extra':1}]:
                c,f=self.controls();c[stage]['native_body_counts']=value
                self.assertEqual(self.classify(c,f),'native_identity_failed')
            c,f=self.controls();c[stage]['native_instruction_paths']=['/other/a.md','/managed/b.md']
            self.assertEqual(self.classify(c,f),'native_identity_failed')
            c,f=self.controls();c[stage]['native_instruction_paths'].reverse()
            self.assertEqual(self.classify(c,f),'observed')
            for paths in [['/managed/a.md','/managed/a.md'],[1,'/managed/b.md']]:
                c,f=self.controls();c[stage]['native_instruction_paths']=paths
                self.assertEqual(self.classify(c,f),'native_identity_failed')

    def test_completed_pin_tool_workspace_and_session_evidence_is_required(self):
        for stage in ['unregistered','installed','resume','updated','removed','reinstalled','outside']:
            for key,value in [('completed',False),('completed',1),('state','timeout'),('tools_observed',True),('tools_observed',None),('pin','other'),('cwd','/other'),('session',' '),('session',None)]:
                c,f=self.controls();c[stage][key]=value
                self.assertNotEqual(self.classify(c,f),'observed')
        c,f=self.controls();c['resume']['session']='other'
        self.assertEqual(self.classify(c,f),'session_identity_failed')
        c,f=self.controls();c['updated']['session']='installed'
        self.assertEqual(self.classify(c,f),'session_identity_failed')
