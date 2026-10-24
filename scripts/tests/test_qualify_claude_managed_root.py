import importlib.util
import json
import os
import subprocess
from pathlib import Path
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-plugin-rules.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class ClaudeManagedRootControls(unittest.TestCase):
    def test_fixture_is_a_copy_only_local_marketplace_outside_discovery(self):
        with tempfile.TemporaryDirectory() as temporary:
            previous_mask = os.umask(0o022)
            try:
                row = q.prepare_claude_managed_root(Path(temporary) / 'fixture')
            finally:
                os.umask(previous_mask)
            self.assertEqual(row['root'].stat().st_mode & 0o777, 0o700)
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
            with mock.patch.dict(os.environ, {'HOME': str(base), 'USERPROFILE': str(base)}), self.assertRaises(ValueError):
                q.prepare_claude_managed_root(Path('~/existing'))
            self.assertEqual((sentinel / 'sentinel').read_bytes(), b'preserve')

    def test_fixture_git_ignores_ambient_repository_and_home_configuration(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            external = base / 'external'
            external.mkdir()
            clean = {key: value for key, value in os.environ.items() if key in ('PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'SystemRoot')}
            clean.update(HOME=str(base / 'baseline-home'), USERPROFILE=str(base / 'baseline-home'))
            def git(*args):
                return subprocess.run(['git', '-C', str(external), *args], env=clean, capture_output=True, text=True, check=True).stdout
            git('init', '-q')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: external repository sentinel')
            sentinel = external / 'sentinel'
            sentinel.write_text('staged sentinel\n')
            git('add', 'sentinel')
            sentinel.write_text('working sentinel\n')
            before = (git('rev-parse', 'HEAD'), (external / '.git/index').read_bytes(), (external / '.git/config').read_bytes(), sentinel.read_bytes())
            home = base / 'ambient-home'
            home.mkdir()
            templates = base / 'ambient-templates'
            templates.mkdir()
            (templates / 'ambient-sentinel').write_text('unexpected template\n')
            (home / '.gitconfig').write_text('[init]\n templateDir = ' + templates.as_posix() + '\n')
            inherited = {**clean, 'CLAUDE_CONFIG_DIR': str(base / 'ambient-claude'), 'COPILOT_HOME': str(base / 'ambient-copilot'), 'CODEX_HOME': str(base / 'ambient-codex'), 'OPENCODE_CONFIG_CONTENT': '{"ambient":true}', 'HOME': str(home), 'USERPROFILE': str(home), 'GIT_DIR': str(external / '.git'), 'GIT_WORK_TREE': str(external), 'GIT_INDEX_FILE': str(external / '.git/index'), 'GIT_CONFIG_COUNT': '1', 'GIT_CONFIG_KEY_0': 'core.worktree', 'GIT_CONFIG_VALUE_0': str(external)}
            real_run = subprocess.run
            with mock.patch.dict(os.environ, inherited, clear=True), mock.patch.object(q.subprocess, 'run', wraps=real_run) as captured:
                row = q.prepare_claude_managed_root(base / 'fixture')
                self.assertEqual({key: os.environ[key] for key in inherited}, inherited)
            self.assertTrue(captured.call_args_list)
            for call in captured.call_args_list:
                child_env = call.kwargs['env']
                self.assertFalse(any(key.startswith('GIT_') for key in child_env))
                self.assertEqual(child_env['HOME'], str(row['root'] / 'home'))
                self.assertEqual(child_env['CLAUDE_CONFIG_DIR'], str(row['root'] / 'home/.claude'))
                self.assertEqual(child_env['COPILOT_HOME'], str(row['root'] / 'home/.copilot'))
                self.assertEqual(child_env['CODEX_HOME'], str(row['root'] / 'home/.codex'))
                self.assertNotIn('OPENCODE_CONFIG_CONTENT', child_env)
            for project in (row['project'], row['user_project']):
                self.assertTrue((project / '.git').is_dir())
                self.assertFalse((project / '.git/ambient-sentinel').exists())
            self.assertEqual((git('rev-parse', 'HEAD'), (external / '.git/index').read_bytes(), (external / '.git/config').read_bytes(), sentinel.read_bytes()), before)
            self.assertFalse((external / '.git/ambient-sentinel').exists())

    def test_repository_relative_root_is_rejected_before_writing(self):
        with tempfile.TemporaryDirectory() as temporary:
            repository = (Path(temporary) / 'repository').resolve()
            repository.mkdir()
            root = repository / 'fixture'
            with mock.patch.object(q, 'REPO', repository), self.assertRaises(ValueError):
                q.prepare_claude_managed_root(root)
            self.assertFalse(root.exists())

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
        self.assertEqual(q.classify_claude_settings(before, before, 0o640, 0o600), 'preservation_failed')
        after = b'{"env":{"MANUAL":"preserve"},"enabledPlugins":{},"extraKnownMarketplaces":{}}\n'
        self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o640), 'empty_object_residue')
        self.assertEqual(q.classify_claude_settings(None, b'{"enabledPlugins":{}}', None, 0o600), 'empty_object_residue')
        for value in ({'env': {'MANUAL': 'changed'}, 'enabledPlugins': {}}, {'env': {'MANUAL': 'preserve'}, 'unknown': {}}, {'env': {'MANUAL': 'preserve'}, 'enabledPlugins': {'unexpected': True}}, {'env': {'MANUAL': 'preserve'}, 'extraKnownMarketplaces': {'unexpected': {'source': 'changed'}}}, {'enabledPlugins': {}}):
            self.assertEqual(q.classify_claude_settings(before, json.dumps(value).encode(), 0o640, 0o640), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o600), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(before, b'{ "env": { "MANUAL": "preserve" } }', 0o640, 0o640), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(before, None, 0o640, None), 'preservation_failed')
        self.assertEqual(q.classify_claude_settings(None, None, None, None), 'unchanged')
        self.assertEqual(q.classify_claude_settings(before, b'invalid', 0o640, 0o640), 'preservation_failed')
        for value in (b'[]', b'true', b'1', b'\"scalar\"', b'null'):
            self.assertEqual(q.classify_claude_settings(before, value, 0o640, 0o640), 'preservation_failed')
            self.assertEqual(q.classify_claude_settings(value, after, 0o640, 0o640), 'preservation_failed')

    def test_existing_registration_values_and_json_types_are_preserved(self):
        cases = [
            (b'{"enabledPlugins":{"manual":true}}', b'{"enabledPlugins":{}}'),
            (b'{"extraKnownMarketplaces":{"manual":{"source":"preserve"}}}', b'{"extraKnownMarketplaces":{}}'),
            (b'{"manual":true}', b'{"manual":1,"enabledPlugins":{}}'),
            (b'{"manual":1}', b'{"manual":true,"extraKnownMarketplaces":{}}'),
            (b'{"manual":false}', b'{"manual":0,"enabledPlugins":{}}'),
            (b'{"manual":[{"nested":true}]}', b'{"manual":[{"nested":1}],"enabledPlugins":{}}'),
            (b'{"manual":1}', b'{"manual":1.0,"enabledPlugins":{}}'),
        ]
        for before, after in cases:
            with self.subTest(before=before, after=after):
                self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o640), 'preservation_failed')

    def test_allowed_residue_preserves_values_independently_of_object_key_order(self):
        cases = [
            (b'{"enabledPlugins":{"manual":true},"manual":{"flag":true,"count":1,"ratio":1.0}}',
             b'{"manual":{"ratio":1.0,"count":1,"flag":true},"extraKnownMarketplaces":{},"enabledPlugins":{"manual":true}}'),
            (b'{"extraKnownMarketplaces":{"manual":{"source":"preserve"}},"env":{"A":"first","B":"second"}}',
             b'{"env":{"B":"second","A":"first"},"enabledPlugins":{},"extraKnownMarketplaces":{"manual":{"source":"preserve"}}}'),
        ]
        for before, after in cases:
            with self.subTest(before=before, after=after):
                self.assertEqual(q.classify_claude_settings(before, after, 0o640, 0o640), 'empty_object_residue')

    def test_absent_settings_cannot_accept_arbitrary_or_falsely_empty_content(self):
        for after in (b'{}', b'{"manual":true}', b'invalid', b'[]', b'null', b'{"enabledPlugins":{"unexpected":true}}', b'{"extraKnownMarketplaces":{"unexpected":{}}}'):
            with self.subTest(after=after):
                self.assertEqual(q.classify_claude_settings(None, after, None, 0o600), 'preservation_failed')
        before = b'{"manual":"preserve"}'
        for value in ([], None, False, 0, ''):
            for key in ('enabledPlugins', 'extraKnownMarketplaces'):
                for original in (None, before):
                    old = {} if original is None else {"manual":"preserve"}
                    after = json.dumps({**old, key:value}).encode()
                    self.assertEqual(q.classify_claude_settings(original, after, None if original is None else 0o640, 0o600 if original is None else 0o640), 'preservation_failed')

    def test_case_aliases_and_additional_client_roots_are_rejected_before_effects(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            for location in ('.Claude/fixture', '.GitHub/fixture', '.CoPiLoT/fixture', '.config/OpenCode/fixture'):
                root = base / location
                with self.subTest(location=location), self.assertRaises(ValueError):
                    q.prepare_claude_managed_root(root)
                self.assertFalse(root.exists())
            override = base / 'configured-client'
            for key in ('CLAUDE_CONFIG_DIR','COPILOT_HOME','CODEX_HOME','OPENCODE_CONFIG_DIR','OPENCODE_CONFIG'):
                root = override / key / 'fixture'
                with mock.patch.dict(os.environ, {key:str(override / key)}), self.subTest(key=key), self.assertRaises(ValueError):
                    q.prepare_claude_managed_root(root)
                self.assertFalse(root.exists())
            root = override / 'xdg' / 'opencode' / 'fixture'
            with mock.patch.dict(os.environ, {'XDG_CONFIG_HOME': str(override / 'xdg')}), self.assertRaises(ValueError):
                q.prepare_claude_managed_root(root)
            self.assertFalse(root.exists())

    def test_ancestor_guidance_or_claude_discovery_rejects_without_writing_guides(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            real_exists, real_symlink = Path.exists, Path.is_symlink
            for name in ('CLAUDE.md','AGENTS.md','.claude','.copilot','.codex','.opencode','.github','.agents/skills','.agents/plugins','.config/opencode'):
                for dangling in (False, True):
                    root = base / 'ancestor' / 'fixture'
                    sentinel = base / name
                    def exists(path):
                        return not dangling if path == sentinel else real_exists(path)
                    def is_symlink(path):
                        return dangling if path == sentinel else real_symlink(path)
                    with mock.patch.object(Path, 'exists', exists), mock.patch.object(Path, 'is_symlink', is_symlink), self.subTest(name=name, dangling=dangling), self.assertRaises(ValueError):
                        q.prepare_claude_managed_root(root)
                    self.assertFalse(root.exists())
                    self.assertFalse(real_exists(sentinel))
                    self.assertFalse(real_symlink(sentinel))

    @unittest.skipIf(os.name == 'nt', 'POSIX symlink control')
    def test_lexical_discovery_ancestor_cannot_redirect_into_a_neutral_location(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary).resolve()
            neutral, hidden = base / 'neutral', base / 'hidden-parent'
            neutral.mkdir()
            hidden.mkdir()
            link = hidden / '.claude'
            link.symlink_to(neutral, target_is_directory=True)
            with self.assertRaises(ValueError):
                q.prepare_claude_managed_root(link / 'fixture')
            self.assertFalse((neutral / 'fixture').exists())
            self.assertTrue(link.is_symlink())
