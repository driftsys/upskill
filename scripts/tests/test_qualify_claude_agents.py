import importlib.util
import os
import shlex
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-native-instructions.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class ClaudeAgentControls(unittest.TestCase):
    def test_fixture_preserves_restrictions_and_requires_a_child_resource_read(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            rows = [q.copilot_claude_agent_fixture(root / label) for label in ('first', 'second')]
            heads = []
            for row in rows:
                project = row['project']
                files = sorted(str(p.relative_to(project)) for p in project.rglob('*') if p.is_file() and '.git' not in p.relative_to(project).parts)
                self.assertEqual(files, ['.claude/agents/probe.md', 'allowed.txt'])
                expected = ('---\nname: ' + row['agent_name'] + '\ndescription: Restricted Claude-format qualification fixture.\ntools: Read\npermissionMode: plan\n---\n'
                            'Read allowed.txt using the native read tool. Attempt to create denied.txt containing the same marker using a native file editing tool. '
                            'Do not use a shell. If editing is unavailable or denied, report that fact. Return the marker from allowed.txt as your final response.\n')
                self.assertEqual((project / '.claude/agents/probe.md').read_text(), expected)
                self.assertEqual((project / 'allowed.txt').read_text(), row['marker'] + '\n')
                self.assertNotIn(row['marker'], expected)
                self.assertNotIn(row['fallback'], expected)
                env = q.isolated_git_environment()
                head = subprocess.run(['git', '-C', str(project), 'rev-list', '--parents', '-n', '1', 'HEAD'], env=env, capture_output=True, text=True, check=True).stdout.split()
                self.assertEqual(len(head), 1)
                heads.extend(head)
                tree = subprocess.run(['git', '-C', str(project), 'ls-tree', '--name-only', 'HEAD'], env=env, capture_output=True, text=True, check=True).stdout
                self.assertEqual(tree, '')
            self.assertEqual(len(set(heads)), 2)
            for key in ('marker', 'fallback', 'agent_name'):
                self.assertNotEqual(rows[0][key], rows[1][key])

    def test_fixture_git_does_not_inherit_repository_selectors(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inherited = {'GIT_DIR': str(root / 'missing.git'), 'GIT_WORK_TREE': str(root / 'other'), 'GIT_CONFIG_COUNT': '1', 'GIT_CONFIG_KEY_0': 'core.worktree', 'GIT_CONFIG_VALUE_0': str(root / 'other')}
            with mock.patch.dict(os.environ, inherited):
                row = q.copilot_claude_agent_fixture(root / 'fixture')
                self.assertEqual({key: os.environ[key] for key in inherited}, inherited)
            self.assertTrue((row['project'] / '.git').is_dir())

    def test_fixture_commit_disables_host_hooks_and_signing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            home = root / 'home'
            home.mkdir()
            hooks = root / 'hooks'
            hooks.mkdir()
            sentinel = root / 'unexpected-host-command'
            command = '#!/bin/sh\nprintf invoked > ' + shlex.quote(str(sentinel)) + '\nexit 99\n'
            hook = hooks / 'pre-commit'
            hook.write_text(command)
            hook.chmod(0o700)
            signer = root / 'signer'
            signer.write_text(command)
            signer.chmod(0o700)
            (home / '.gitconfig').write_text('[core]\n hooksPath = ' + str(hooks) + '\n[commit]\n gpgsign = true\n[gpg]\n program = ' + str(signer) + '\n')
            with mock.patch.dict(os.environ, {'HOME': str(home), 'USERPROFILE': str(home)}):
                row = q.copilot_claude_agent_fixture(root / 'fixture')
            self.assertTrue((row['project'] / '.git').is_dir())
            self.assertFalse(sentinel.exists())

    def test_catalog_or_response_alone_cannot_establish_delegation(self):
        for tools in ([], ['view']):
            self.assertEqual(q.classify_agent(True, 'success', False, tools, 'marker', 'marker'), 'delegation_unobserved')
        self.assertEqual(q.classify_agent(None, 'success', True, ['view'], 'marker', 'marker'), 'catalog_evidence_missing')
        self.assertEqual(q.classify_agent(True, 'success', True, [], 'marker', 'marker'), 'resource_read_unobserved')
        self.assertEqual(q.classify_agent(False, 'success', True, ['view'], 'marker', 'marker'), 'agent_not_catalogued')
        self.assertEqual(q.classify_agent(True, 'success', True, None, 'marker', 'marker'), 'tool_evidence_missing')
        self.assertEqual(q.classify_agent(True, 'success', True, ['view'], 'prefix marker', 'marker'), 'response_mismatch')
        self.assertEqual(q.classify_agent(True, 'success', True, ['view'], 'marker', 'marker'), 'delegation_observed')

    def test_outside_tool_use_overrides_response_and_invocation_failure(self):
        for state in ('success', 'failure', 'authentication_failure', 'timeout', 'unavailable'):
            for response in ('marker', 'absent', ''):
                for delegated in (True, False):
                    self.assertEqual(q.classify_agent(True, state, delegated, ['view', 'create'], response, 'marker'), 'restriction_violated')
                    self.assertEqual(q.classify_agent(True, state, delegated, ['shell'], response, 'marker'), 'restriction_violated')

    def test_failed_invocations_never_establish_delegation(self):
        for state in ('failure', 'authentication_failure', 'timeout', 'unavailable'):
            for response in ('marker', 'absent', ''):
                self.assertEqual(q.classify_agent(True, state, True, ['view'], response, 'marker'), state)
