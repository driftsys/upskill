import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class ClaudeManagedRootControls(unittest.TestCase):
    def test_fixture_is_a_copy_only_local_marketplace_outside_discovery(self):
        with tempfile.TemporaryDirectory() as temporary:
            row = q.prepare_claude_managed_root(Path(temporary) / 'fixture')
            managed = row['managed_root']
            market = json.loads((managed / '.claude-plugin/marketplace.json').read_text())
            self.assertEqual(market, {'name': row['market_name'], 'owner': {'name': 'Qualification Fixture'}, 'plugins': [{'name': row['plugin_name'], 'source': './packages/' + row['plugin_name']}]})
            package = managed / 'packages' / row['plugin_name']
            self.assertEqual(row['package'], package)
            self.assertEqual(json.loads((package / '.claude-plugin/plugin.json').read_text()), {'name': row['plugin_name'], 'version': '0.0.0', 'description': 'Disposable managed-root qualification fixture.'})
            expected = '---\nname: ' + row['skill_name'] + '\ndescription: Explicit managed-root qualification skill.\n---\nReturn exactly ' + row['initial_marker'] + ' and nothing else. Do not use any other tools.\n'
            self.assertEqual(row['skill'].read_text(), expected)
            self.assertEqual(len({row[k] for k in ('initial_marker', 'edited_marker', 'fallback')}), 3)
            paths = list(row['root'].rglob('*'))
            self.assertFalse(any(p.is_symlink() for p in paths))
            self.assertFalse(any(p.name in ('AGENTS.md', 'CLAUDE.md') for p in paths))
            self.assertEqual([p for p in paths if p.name == 'SKILL.md'], [row['skill']])
            self.assertFalse(managed.is_relative_to(row['project']))
            self.assertFalse(managed.is_relative_to(row['user_project']))
            for project in (row['project'], row['user_project']):
                self.assertEqual(json.loads((project / '.claude/settings.json').read_text()), {'env': {'UPSKILL_Q6_MANUAL': row['manual']}})
                self.assertEqual((project / '.claude/settings.json').stat().st_mode & 0o777, 0o640)
            import subprocess
            heads = [subprocess.run(['git', '-C', str(project), 'rev-list', '--parents', '-n', '1', 'HEAD'], env=q.fixture_environment(row['root']), capture_output=True, text=True, check=True).stdout.split() for project in (row['project'], row['user_project'])]
            self.assertTrue(all(len(head) == 1 for head in heads))
            self.assertNotEqual(heads[0], heads[1])
            other = q.prepare_claude_managed_root(Path(temporary) / 'second-fixture')
            for key in ('plugin_name', 'market_name', 'skill_name', 'initial_marker', 'edited_marker', 'fallback', 'manual'):
                self.assertNotEqual(row[key], other[key])

    def test_discovery_roots_and_existing_directories_are_rejected_before_writing(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for location in ('.claude/fixture', '.github/fixture', '.codex/fixture', '.opencode/fixture', '.agents/skills/fixture', '.agents/plugins/fixture'):
                root = base / location
                with self.assertRaises(ValueError):
                    q.prepare_claude_managed_root(root)
                self.assertFalse(root.exists())
            sentinel = base / 'existing'
            sentinel.mkdir()
            (sentinel / 'sentinel').write_bytes(b'preserve')
            with self.assertRaises(ValueError):
                q.prepare_claude_managed_root(sentinel)
            self.assertEqual((sentinel / 'sentinel').read_bytes(), b'preserve')

    @unittest.skipIf(os.name == 'nt', 'POSIX symlink control')
    def test_dangling_root_symlink_does_not_redirect_fixture_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            target = base / 'unexpected'
            link = base / 'fixture'
            link.symlink_to(target, target_is_directory=True)
            with self.assertRaises(ValueError):
                q.prepare_claude_managed_root(link)
            self.assertFalse(target.exists())
            self.assertTrue(link.is_symlink())

    def test_only_added_empty_native_registration_objects_are_allowed_residue(self):
        before = b'{"env":{"MANUAL":"preserve"}}\n'
        self.assertEqual(q.classify_claude_settings(before, before, 0o640, 0o640), 'unchanged')
        after = b'{"env":{"MANUAL":"preserve"},"enabledPlugins":{},"extraKnownMarketplaces":{}}\n'
        self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o640), 'empty_object_residue')
        self.assertEqual(q.classify_claude_settings(None, b'{"enabledPlugins":{}}', None, 0o600), 'empty_object_residue')
        for value in ({'env': {'MANUAL': 'changed'}, 'enabledPlugins': {}}, {'env': {'MANUAL': 'preserve'}, 'unknown': {}}, {'env': {'MANUAL': 'preserve'}, 'enabledPlugins': {'unexpected': True}}, {'enabledPlugins': {}}):
            self.assertEqual(q.classify_claude_settings(before, json.dumps(value).encode(), 0o640, 0o640), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o600), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(before, b'{ "env": { "MANUAL": "preserve" } }', 0o640, 0o640), 'settings_bytes_changed')
        self.assertEqual(q.classify_claude_settings(before, None, 0o640, None), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(None, None, None, None), 'unchanged')
        self.assertEqual(q.classify_claude_settings(before, b'invalid', 0o640, 0o640), 'invalid_settings')
