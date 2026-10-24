"""Offline Q7 package-owned Codex agents and boundary evidence; no client calls."""
import importlib.util
import json
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
            self.assertEqual(agent['name'],f['agent_name'])
            self.assertIn(str(f['resource']),agent['developer_instructions'])
            self.assertIn('attempt',agent['developer_instructions'])
            self.assertNotIn(f['marker'],agent['developer_instructions'])
            self.assertNotIn('sandbox_mode',agent)
            self.assertNotIn('approval_policy',agent)
            config=tomllib.loads((f['project']/'.codex/config.toml').read_text())
            self.assertEqual(config['agents'][f['agent_name']]['config_file'],str(f['agent_file']))
            self.assertIs(config['agents']['enabled'],True)
            self.assertEqual(config['agents']['max_threads'],1)
            positive=tomllib.loads((f['positive_project']/'.codex/config.toml').read_text())
            self.assertIn(f['native_marker'],positive['developer_instructions'])
            manifest=json.loads((f['package']/'.codex-plugin/plugin.json').read_text())
            self.assertNotIn('hooks',manifest)
            market=json.loads((f['marketplace']/'.agents/plugins/marketplace.json').read_text())
            self.assertEqual(market['plugins'][0]['source']['path'],'./plugins/'+f['plugin_name'])
            self.assertEqual(f['resource'].read_text(),f['marker']+'\n')
            self.assertFalse(f['forbidden_file'].exists())
            heads=[subprocess.check_output(['git','-C',str(f[k]),'rev-parse','HEAD']) for k in ('project','positive_project')]
            self.assertNotEqual(*heads)
            for p in f['root'].rglob('*'):
                self.assertFalse(p.is_symlink())
                self.assertNotIn(p.name,('AGENTS.md','CLAUDE.md','AGENTS.override.md'))

    def test_root_protection_and_marker_freshness(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for p in (root,*(root/s/'fixture' for s in ('.claude','.github','.codex','.opencode','.copilot','.config/opencode','.agents/skills','.agents/plugins'))):
                with self.assertRaises(ValueError):self.q.prepare_codex_agent_registration(p)
            a=self.q.prepare_codex_agent_registration(root/'a');b=self.q.prepare_codex_agent_registration(root/'b')
            for key in ('marker','native_marker','agent_name','plugin_name'):self.assertNotEqual(a[key],b[key])
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
                self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')
        for key,value in (('parent_sandbox','workspace-write'),('child_sandbox','danger-full-access'),('response',{'marker':'M','write':'allowed'}),('response',{'marker':'M','write':'denied','extra':True})):
            c,f=self.controls();c['registered'][key]=value
            self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')

    def test_plugin_only_needs_installed_catalog_and_no_registration(self):
        for key in ('native_catalog_observed','plugin_installed','external_registration_absent'):
            for value in (False,None,1):
                c,f=self.controls();c['plugin_only'][key]=value
                self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')
        for value in (True,None,0):
            c,f=self.controls();c['plugin_only']['candidate_present']=value
            self.assertNotEqual(self.q.classify_codex_agent_registration(c,f),'observed')

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
