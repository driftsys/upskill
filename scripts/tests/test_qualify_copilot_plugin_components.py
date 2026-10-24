"""Offline Q3 component fixtures and classification; never invoke Copilot."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


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

    def observations(self):
        f = {'pin': '1.0.103', 'native_markers': {'project': 'N'}, 'plain_marker': 'P', 'scoped_marker': 'S', 'agent_marker': 'A'}
        def row(rules):return {'pin': '1.0.103', 'state': 'success', 'completed': True, 'response': {'native': 'N', 'rules': rules}}
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
