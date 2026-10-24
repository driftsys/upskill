"""Offline Q4 shell hook and lifecycle classification; no client invocation."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


class ClaudeRuleHookTests(unittest.TestCase):
    def setUp(self):
        path = Path(__file__).parents[1] / 'qualify-plugin-rules.py'
        spec = importlib.util.spec_from_file_location('q4', path)
        self.q = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.q)

    def test_package_index_and_shell_hook_deliver_one_independent_body(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.prepare_claude_rule_hook(Path(tmp) / 'fixture')
            package = f['package']
            self.assertEqual(f['pin'], '2.1.308')
            native_text = (f['positive_project']/'.claude/rules/native-positive.md').read_text()
            self.assertEqual(native_text, 'For the unscoped hook question return exactly ' + f['native_marker'] + ' and nothing else. Do not use tools.\n')
            self.assertNotIn(f['hook_marker'], native_text)
            self.assertNotIn(f['fallback'], native_text)
            self.assertFalse((f['project']/'.claude/rules').exists())
            self.assertEqual((package / 'rules/index').read_text(), 'rule-a.json\n')
            manifest = json.loads((package / '.claude-plugin/plugin.json').read_text())
            self.assertEqual(manifest['name'], f['plugin_name'])
            hooks = json.loads((package / 'hooks/hooks.json').read_text())
            self.assertEqual(hooks, {'hooks': {'SessionStart': [{'hooks': [{'type': 'command', 'command': 'sh "${CLAUDE_PLUGIN_ROOT}/hook.sh"'}]}]}})
            expected = {'hookSpecificOutput': {'hookEventName': 'SessionStart', 'additionalContext': 'For the unscoped hook question return exactly ' + f['hook_marker'] + ' and nothing else. Do not use tools.\n'}}
            env = self.q.fixture_environment(f['root']);env['CLAUDE_PLUGIN_ROOT'] = str(package)
            for source in ('startup', 'resume', 'compact'):
                event = {'hook_event_name': 'SessionStart', 'source': source, 'session_id': 'offline', 'cwd': str(f['project'])}
                result = subprocess.run(['/bin/sh', str(package / 'hook.sh')], input=json.dumps(event), capture_output=True, text=True, env=env, check=True)
                self.assertEqual(json.loads(result.stdout), expected)
            events = [json.loads(s) for s in f['trace'].read_text().split('\x1e') if s.strip()]
            self.assertEqual(events, [{'hook_event_name': 'SessionStart', 'source': source, 'session_id': 'offline', 'cwd': str(f['project'])} for source in ('startup', 'resume', 'compact')])
            market = json.loads((f['marketplace']/'.claude-plugin/marketplace.json').read_text())
            self.assertEqual(market, {'name': f['market_name'], 'owner': {'name': 'Qualification Fixture'}, 'plugins': [{'name': f['plugin_name'], 'source': './packages/' + f['plugin_name']}]})
            settings = f['project']/'.claude/settings.json'
            manual = json.loads(settings.read_text())
            self.assertEqual(set(manual), {'env'})
            self.assertEqual(set(manual['env']), {'UPSKILL_Q4_MANUAL'})
            self.assertTrue(manual['env']['UPSKILL_Q4_MANUAL'].startswith('MANUAL_'))
            self.assertEqual(settings.stat().st_mode & 0o777, 0o640)
            for path in f['root'].rglob('*'):
                self.assertFalse(path.is_symlink())
                self.assertNotIn(path.name, ('AGENTS.md', 'AGENTS.override.md', 'CLAUDE.md', 'CLAUDE.local.md', 'copilot-instructions.md'))
            commits = [subprocess.check_output(['git', '-C', str(p), 'rev-parse', 'HEAD'], env=env) for p in (f['project'], f['positive_project'])]
            self.assertNotEqual(*commits)

    def test_existing_and_discovery_roots_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for p in (root, *(root / s / 'fixture' for s in ('.claude', '.github', '.codex', '.opencode', '.agents/skills', '.agents/plugins'))):
                with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(p)
            link = root / 'dangling';link.symlink_to(root / 'missing')
            with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(link)

    def test_invalid_index_cannot_escape_package(self):
        with tempfile.TemporaryDirectory() as tmp:
            f = self.q.prepare_claude_rule_hook(Path(tmp) / 'fixture')
            env = self.q.fixture_environment(f['root']);env['CLAUDE_PLUGIN_ROOT'] = str(f['package'])
            # Every invalid index resolves to readable bytes if the allowlist is removed.
            for bad in ('../outside.json', '/outside.json', 'rule-a.json\nrule-b.json'):
                outside = f['package'] / ('rules/' + bad)
                outside.parent.mkdir(parents=True, exist_ok=True)
                outside.write_text('OUTSIDE_SENTINEL\n')
                (f['package'] / 'rules/index').write_text(bad + '\n')
                before = f['trace'].read_bytes() if f['trace'].exists() else None
                result = subprocess.run(['/bin/sh', str(f['package'] / 'hook.sh')], input='{}', capture_output=True, text=True, env=env)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, '')
                after = f['trace'].read_bytes() if f['trace'].exists() else None
                self.assertEqual(after, before)

    def controls(self):
        f = {'pin': '2.1.308', 'native_marker': 'N', 'hook_marker': 'H', 'fallback': 'F'}
        c = {k: {'pin': '2.1.308', 'state': 'success', 'completed': True, 'response': 'H', 'events': [{'source': source, 'session_id': 'same'}]} for k, source in [('installed','startup'),('resume','resume'),('compact','compact')]}
        c['compact']['native_compaction_completed'] = True
        c['native_positive'] = {'pin': '2.1.308', 'state': 'success', 'completed': True, 'response': 'N'}
        c.update({k: {'pin':'2.1.308','state':'success','completed':True,'response':'F','events':[]} for k in ('disabled','uninstalled')})
        for phase,row in c.items():
            row.update(cwd='/fixture/positive' if phase=='native_positive' else '/fixture/project', native_tools=[], model_tool_uses=0)
        f.update(project='/fixture/project',positive_project='/fixture/positive')
        for phase in ('installed','resume','compact'):
            c[phase]['events'][0].update(hook_event_name='SessionStart',cwd='/fixture/project')
        return c, f

    def test_lifecycle_requires_exact_markers_and_one_event(self):
        c, f = self.controls();self.assertEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
        for phase in ('installed','resume','compact','disabled','uninstalled'):
            c, f = self.controls();c[phase]['response'] = 'wrong'
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
            c, f = self.controls();c[phase]['events'].append({'source':'startup','session_id':'same'})
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')

    def test_failed_positive_never_counts_as_absence(self):
        c, f = self.controls();c['native_positive']['state'] = 'authentication_failure'
        self.assertEqual(self.q.classify_claude_rule_hook(c, f), 'positive_control_failed')
        for wrong in ('H','F','wrong'):
            c, f = self.controls();c['native_positive']['response'] = wrong
            self.assertEqual(self.q.classify_claude_rule_hook(c, f), 'positive_control_failed')
        c, f = self.controls();c['installed']['response'] = 'F'
        self.assertEqual(self.q.classify_claude_rule_hook(c, f), 'installed_positive_failed')

    def test_pin_completion_session_and_compaction_evidence_are_required(self):
        for phase in ('native_positive','installed','resume','compact','disabled','uninstalled'):
            for key, value in [('pin','other'),('completed',False),('completed',1),('state','timeout')]:
                c, f = self.controls();c[phase][key] = value
                self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
        for key in ('native_compaction_completed',):
            c, f = self.controls();c['compact'][key] = False
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
        for phase in ('installed','resume','compact'):
            c, f = self.controls();c[phase]['events'][0]['session_id'] = 'different'
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
            c, f = self.controls();c[phase]['events'][0]['source'] = 'resume' if phase == 'installed' else 'startup'
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')
            c, f = self.controls();del c[phase]['events'][0]['source']
            self.assertNotEqual(self.q.classify_claude_rule_hook(c, f), 'observed')

    def test_markers_are_fresh_and_repository_home_roots_are_protected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fixtures = [self.q.prepare_claude_rule_hook(root/name) for name in ('one','two')]
            for field in ('native_marker','hook_marker','fallback','plugin_name'):
                self.assertNotEqual(fixtures[0][field],fixtures[1][field])
            for f in fixtures:
                self.assertEqual(len({f['native_marker'],f['hook_marker'],f['fallback']}),3)
            with mock.patch.object(self.q,'REPO',root.resolve()/'repository'):
                with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(root/'repository/fixture')
            with mock.patch.object(Path,'home',return_value=root.resolve()/'home'):
                with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(root/'home')

    def test_missing_evidence_and_matching_invalid_sessions_never_qualify(self):
        for phase in ('native_positive','installed','resume','compact','disabled','uninstalled'):
            for key in ('pin','state','completed','response'):
                c,f=self.controls();del c[phase][key]
                self.assertNotEqual(self.q.classify_claude_rule_hook(c,f),'observed')
        for changed in (None,'',' ',1,[]):
            c,f=self.controls()
            for phase in ('installed','resume','compact'):c[phase]['events'][0]['session_id']=changed
            self.assertNotEqual(self.q.classify_claude_rule_hook(c,f),'observed')
        for changed in (None,False,1,'yes'):
            c,f=self.controls();c['compact']['native_compaction_completed']=changed
            self.assertNotEqual(self.q.classify_claude_rule_hook(c,f),'observed')

    def test_zero_missing_and_swapped_active_callbacks_are_rejected(self):
        for phase in ('installed','resume','compact'):
            c,f=self.controls();c[phase]['events']=[]
            self.assertEqual(self.q.classify_claude_rule_hook(c,f),'lifecycle_control_failed')
            c,f=self.controls();del c[phase]['events']
            self.assertEqual(self.q.classify_claude_rule_hook(c,f),'lifecycle_control_failed')
        for phase,source in (('resume','compact'),('compact','resume')):
            c,f=self.controls();c[phase]['events'][0]['source']=source
            self.assertEqual(self.q.classify_claude_rule_hook(c,f),'lifecycle_control_failed')

    def test_absence_requires_explicit_empty_callback_evidence(self):
        for phase in ('disabled','uninstalled'):
            for value in (None,{},'missing'):
                c,f=self.controls()
                if value=='missing':del c[phase]['events']
                else:c[phase]['events']=value
                self.assertEqual(self.q.classify_claude_rule_hook(c,f),'absence_control_failed')

    def test_every_phase_requires_observed_tool_and_workspace_boundaries(self):
        for phase in ('native_positive','installed','resume','compact','disabled','uninstalled'):
            expected = 'positive_control_failed' if phase=='native_positive' else 'installed_positive_failed' if phase=='installed' else 'absence_control_failed' if phase in ('disabled','uninstalled') else 'lifecycle_control_failed'
            for key,value in [('native_tools',['Read']),('native_tools',None),('model_tool_uses',1),('model_tool_uses',False),('model_tool_uses',None),('cwd','/other-project'),('cwd',None)]:
                with self.subTest(phase=phase,key=key,value=value):
                    c,f=self.controls();c[phase][key]=value
                    self.assertEqual(self.q.classify_claude_rule_hook(c,f),expected)
            for key in ('model_tool_uses','cwd'):
                c,f=self.controls();del c[phase][key]
                self.assertEqual(self.q.classify_claude_rule_hook(c,f),expected)

    def test_active_callback_shapes_names_and_projects_are_required(self):
        for phase in ('installed','resume','compact'):
            for value in (None,{0:{'source':'startup','session_id':'same'}},[None],['bad']):
                c,f=self.controls();c[phase]['events']=value
                self.assertEqual(self.q.classify_claude_rule_hook(c,f),'lifecycle_control_failed')
            for key,value in [('hook_event_name','SessionEnd'),('cwd','/other-project')]:
                c,f=self.controls();c[phase]['events'][0][key]=value
                self.assertEqual(self.q.classify_claude_rule_hook(c,f),'lifecycle_control_failed')

    def test_retained_packet_replays_observed_callback_boundaries(self):
        packet=Path(__file__).resolve().parents[2]/'docs/technotes/evidence/2026-10-24-claude-rule-hooks'
        summary=json.loads((packet/'summary.json').read_text());fixture=json.loads((packet/'fixture.json').read_text())
        self.assertEqual(self.q.classify_claude_rule_hook({row['label']:row for row in summary['controls']},fixture),'observed')
        self.assertEqual(summary['classification'],'observed')

    def test_trace_failure_cannot_emit_unrecorded_hook_context(self):
        with tempfile.TemporaryDirectory() as temporary:
            f=self.q.prepare_claude_rule_hook(Path(temporary)/'fixture')
            f['trace'].mkdir()
            env=self.q.fixture_environment(f['root']);env['CLAUDE_PLUGIN_ROOT']=str(f['package'])
            result=subprocess.run(['/bin/sh',str(f['package']/'hook.sh')],input='{}',capture_output=True,text=True,env=env)
            self.assertNotEqual(result.returncode,0)
            self.assertEqual(result.stdout,'')

    def test_configured_roots_and_ancestor_guidance_are_rejected_before_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary).resolve()
            configured=root/'ordinary-client-profile';configured.mkdir()
            for variable in ('CLAUDE_CONFIG_DIR','CODEX_HOME','COPILOT_HOME','OPENCODE_CONFIG_DIR'):
                candidate=configured/'fixture'
                with mock.patch.dict(os.environ,{variable:str(configured)}):
                    with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(candidate)
                self.assertFalse(candidate.exists())
            actual_exists=Path.exists
            for name in ('CLAUDE.md','AGENTS.md','AGENTS.override.md'):
                guide=root/name
                def exists(path):return path==guide or actual_exists(path)
                with mock.patch.object(Path,'exists',exists), mock.patch.object(Path,'mkdir',side_effect=AssertionError('Reject before writes')):
                    with self.assertRaises(ValueError):self.q.prepare_claude_rule_hook(root/'fixture')
                self.assertFalse((root/'fixture').exists())
