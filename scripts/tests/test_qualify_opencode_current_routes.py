"""Offline Q5 fixture and current-release classification; no client calls."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


class OpenCodeCurrentRoutesTests(unittest.TestCase):
    def setUp(self):
        path = Path(__file__).parents[1] / 'qualify-plugin-rules.py'
        spec = importlib.util.spec_from_file_location('q5', path)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)

    def test_neutral_packages_native_positive_and_explicit_skill_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.prepare_opencode_current_routes(Path(tmp)/'fixture')
            self.assertEqual(f['pin'], '2.1.4')
            self.assertEqual(len(set(f['rule_markers'] + [f['native_marker'],f['resource_marker'],f['fallback']])),5)
            config=json.loads((f['project']/'opencode.json').read_text())
            self.assertEqual(config['plugins'], [str(p) for p in f['packages']])
            self.assertEqual(config['skills'], [str(f['skill_source'])])
            self.assertEqual(config['permissions'], [{'action':'*','resource':'*','effect':'deny'},{'action':'skill','resource':f['skill_id'],'effect':'allow'},{'action':'read','resource':str(f['resource_path']),'effect':'allow'}])
            entry=(f['skill_source']/f['skill_id']/'SKILL.md').read_text()
            resource=f['skill_source']/f['skill_id']/'references/procedure.md'
            self.assertNotIn(f['resource_marker'],entry)
            self.assertIn('Read references/procedure.md relative to this skill',entry)
            self.assertIn(f['fallback'],entry)
            self.assertEqual(resource.read_text(),'Return exactly '+f['resource_marker']+' and no other text.\n')
            positive=json.loads((f['positive_project']/'opencode.json').read_text())
            self.assertEqual(positive['plugins'],[])
            self.assertEqual(positive['permissions'],[{'action':'*','resource':'*','effect':'deny'}])
            self.assertEqual(positive['instructions'],[str(f['positive_project']/'native-positive.md')])
            self.assertIn(f['native_marker'],(f['positive_project']/'native-positive.md').read_text())
            for i, package in enumerate(f['packages']):
                self.assertEqual(json.loads((package/'fixture.json').read_text()),{'project':str(f['project']),'rule':('a','b')[i],'trace':str(f['trace']),'agent':'build'})
                self.assertEqual(json.loads((package/'package.json').read_text())['exports'],'./index.ts')
                self.assertEqual((package/'rule.txt').read_text(),'For the plugin qualification question include exactly one occurrence of '+f['rule_markers'][i]+' in the rules array. Return only the JSON object with a rules array.\n')
                self.assertEqual(json.loads((package/'package.json').read_text())['dependencies'], {'@opencode/plugin':'2.1.4'})
                self.assertIn('ctx.session.hook("context"',(package/'index.ts').read_text())
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name,('CLAUDE.md','AGENTS.md','AGENTS.override.md'))
            heads=[subprocess.check_output(['git','-C',str(p),'rev-parse','HEAD']) for p in (f['project'],f['positive_project'])]
            self.assertNotEqual(*heads)

    def test_root_protection_and_fresh_markers(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for p in (root, *(root/s/'fixture' for s in ('.claude','.github','.codex','.opencode','.agents/skills','.agents/plugins'))):
                with self.assertRaises(ValueError):self.q.prepare_opencode_current_routes(p)
            a=self.q.prepare_opencode_current_routes(root/'a');b=self.q.prepare_opencode_current_routes(root/'b')
            for key in ('native_marker','resource_marker','fallback','rule_markers','skill_id'):self.assertNotEqual(a[key],b[key])
            with mock.patch.object(self.q,'REPO',root.resolve()/'repo'):
                with self.assertRaises(ValueError):self.q.prepare_opencode_current_routes(root/'repo/new')
            with mock.patch.object(Path,'home',return_value=root.resolve()/'home'):
                with self.assertRaises(ValueError):self.q.prepare_opencode_current_routes(root/'home')

    def controls(self):
        f={'pin':'2.1.4','native_marker':'N','rule_markers':['A','B'],'resource_marker':'R','fallback':'F'}
        def row(response):return dict(pin='2.1.4',state='success',completed=True,response=response)
        c={'native_positive':row('N'),'context':row({'rules':['A','B']}),'resume':row({'rules':['A','B']}),'skill':row('R'),'plugin_add':{'pin':'2.1.4','recorded':True,'exit_code':2}}
        for phase in ('context','resume'):c[phase].update(session='same',events=[{'rule':'a','session':'same'},{'rule':'b','session':'same'}])
        c['skill'].update(native_invocation=True,resource_read=True)
        return c,f

    def test_complete_observations_and_plugin_add_recorded_either_way(self):
        c,f=self.controls();self.assertEqual(self.q.classify_opencode_current_routes(c,f),'observed')
        for code in (0,1,2):
            c,f=self.controls();c['plugin_add']['exit_code']=code
            self.assertEqual(self.q.classify_opencode_current_routes(c,f),'observed')

    def test_failed_positive_is_never_a_negative_result(self):
        for key,value in (('state','authentication_failure'),('response','F'),('completed',False),('pin','other')):
            c,f=self.controls();c['native_positive'][key]=value
            self.assertEqual(self.q.classify_opencode_current_routes(c,f),'positive_control_failed')

    def test_exact_rule_responses_and_individual_counts(self):
        for phase in ('context','resume'):
            for response in ({'rules':['A','A','B']},{'rules':['A']},{'rules':['A','B'],'extra':True},'A B'):
                c,f=self.controls();c[phase]['response']=response
                self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')
            for events in ([],[{'rule':'a','session':'same'}]*2,[{'rule':'a','session':'same'},{'rule':'b','session':'other'}]):
                c,f=self.controls();c[phase]['events']=events
                self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')

    def test_pin_completion_and_correlated_resume_identity(self):
        for phase in ('native_positive','context','resume','skill'):
            for key,value in (('pin','other'),('completed',1),('completed',None),('state','timeout')):
                c,f=self.controls();c[phase][key]=value
                self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')
        for invalid in ('',None,1,[], 'different'):
            c,f=self.controls();c['resume']['session']=invalid
            self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')
        c,f=self.controls();c['resume']['session']='foreign'
        for event in c['resume']['events']:event['session']='foreign'
        self.assertEqual(self.q.classify_opencode_current_routes(c,f),'context_control_failed')
        for invalid in ('',None,1,[]):
            c,f=self.controls()
            for phase in ('context','resume'):
                c[phase]['session']=invalid
                for e in c[phase]['events']:e['session']=invalid
            self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')

    def test_skill_needs_native_invocation_and_real_resource_read(self):
        for key,value in (('response','F'),('native_invocation',False),('native_invocation',1),('resource_read',False),('resource_read',1)):
            c,f=self.controls();c['skill'][key]=value
            self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')

    def test_missing_controls_and_unrecorded_install_verb_cannot_qualify(self):
        c,f=self.controls()
        for phase in c:
            c,f=self.controls();del c[phase]
            self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')
        for key,value in (('pin','other'),('recorded',1),('recorded',False),('exit_code',None),('exit_code',False)):
            c,f=self.controls();c['plugin_add'][key]=value
            self.assertNotEqual(self.q.classify_opencode_current_routes(c,f),'observed')
