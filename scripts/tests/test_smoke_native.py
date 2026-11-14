import argparse
import contextlib
import importlib.util
import io
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).parents[2]
spec = importlib.util.spec_from_file_location('qualification', ROOT / 'scripts' / 'qualify-native-instructions.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)

BINARY = ROOT / 'target' / 'debug' / 'upskill'


def smoke_args(**overrides):
    values = dict(smoke=True, run=True, upskill=Path('/fake/upskill'), clients=['claude'],
                  kinds=list(q.SMOKE_KINDS), scopes=['project'], allow_real_home=False, timeout=5)
    values.update(overrides)
    return argparse.Namespace(**values)


class FakeRunner:
    """Records every command. `answer(command)` returns the scripted output."""

    def __init__(self, answer=None):
        self.commands = []
        self.answer = answer

    def __call__(self, command, cwd, timeout, env):
        self.commands.append([str(part) for part in command])
        if self.answer:
            scripted = self.answer(self.commands[-1])
            if scripted is not None:
                return scripted
        if command[1:] == ['--version']:
            return {'state': 'success', 'stdout': '1.2.3 (Fake Client)\n'}
        return {'state': 'success', 'stdout': ''}


class SmokeSourceTest(unittest.TestCase):
    def test_source_holds_one_item_of_each_kind_with_its_own_marker(self):
        markers = {'rule': 'MARK_RULE_1', 'skill': 'MARK_SKILL_2', 'agent': 'MARK_AGENT_3'}
        with tempfile.TemporaryDirectory() as tmp:
            source = q.smoke_source(Path(tmp), 'upskill-smoke-abc', markers)
            files = {'rule': source / 'rules' / 'upskill-smoke-abc' / 'RULE.md',
                     'skill': source / 'skills' / 'upskill-smoke-abc' / 'SKILL.md',
                     'agent': source / 'agents' / 'upskill-smoke-abc' / 'AGENT.md'}
            for kind, path in files.items():
                text = path.read_text()
                self.assertIn('name: upskill-smoke-abc', text)
                self.assertIn(markers[kind], text)
                for other, marker in markers.items():
                    if other != kind:
                        self.assertNotIn(marker, text)


class SmokeClassifyTest(unittest.TestCase):
    def test_a_response_with_the_marker_passes(self):
        self.assertEqual(q.classify_smoke('The marker is M1.', 'M1', ['M2', 'M3']), 'pass')

    def test_a_response_with_the_marker_of_another_kind_fails(self):
        self.assertEqual(q.classify_smoke('M1 and M2', 'M1', ['M2', 'M3']), 'fail')
        self.assertEqual(q.classify_smoke('M2', 'M1', ['M2', 'M3']), 'fail')
        self.assertEqual(q.classify_smoke('', 'M1', ['M2', 'M3']), 'fail')


class SmokeRunTest(unittest.TestCase):
    def test_without_run_no_command_is_started(self):
        runner = FakeRunner()
        results = q.run_smoke(smoke_args(run=False, scopes=['project', 'global'], allow_real_home=True), runner)
        self.assertEqual(runner.commands, [])
        self.assertEqual(len(results), 6)
        self.assertTrue(all(cell['result'] == 'not run' for cell in results))

    def test_global_cells_are_unavailable_without_allow_real_home(self):
        runner = FakeRunner()
        results = q.run_smoke(smoke_args(scopes=['global']), runner)
        self.assertEqual({cell['result'] for cell in results}, {'unavailable'})
        self.assertIn('--allow-real-home', results[0]['reason'])
        self.assertFalse([c for c in runner.commands if '--global' in c])

    def test_global_items_are_removed_after_a_timeout(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                raise subprocess.TimeoutExpired(command, 5)

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual({cell['result'] for cell in results}, {'timeout'})
        last = runner.commands[-1]
        self.assertEqual(last[1:4], ['remove', '--global', '--yes'])
        self.assertTrue(last[4].startswith('upskill-smoke-'))
        add = next(c for c in runner.commands if c[1] == 'add')
        self.assertIn('--global', add)

    def test_global_items_are_removed_after_an_exception(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                raise RuntimeError('boom')

        runner = FakeRunner(answer)
        with self.assertRaises(RuntimeError):
            q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual(runner.commands[-1][1:4], ['remove', '--global', '--yes'])

    def test_a_relative_binary_path_is_made_absolute(self):
        runner = FakeRunner()
        q.run_smoke(smoke_args(upskill=Path('target/release/upskill'), scopes=['global'], allow_real_home=True), runner)
        for command in runner.commands:
            if command[1] in ('add', 'remove'):
                self.assertTrue(os.path.isabs(command[0]), command)
                self.assertEqual(command[0], str(Path('target/release/upskill').resolve()))

    def test_a_failed_install_is_reported_as_install_failed(self):
        def answer(command):
            if command[1] == 'add':
                return {'state': 'failure', 'stdout': ''}

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(), runner)
        self.assertEqual({cell['result'] for cell in results}, {'install-failed'})
        self.assertTrue(all(cell['reason'] for cell in results))
        self.assertFalse([c for c in runner.commands if c[0] == 'claude' and '--version' not in c])
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(), FakeRunner(answer))
        self.assertEqual(out.getvalue().count('note: '), 3)
        self.assertIn('claude rule experimental', out.getvalue())

    def test_a_failed_global_removal_is_reported(self):
        def answer(command):
            if command[1] == 'remove':
                return {'state': 'failure', 'stdout': ''}

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), FakeRunner(answer))
        warnings = [line for line in out.getvalue().splitlines() if line.startswith('warning: global items upskill-smoke-')]
        self.assertEqual(len(warnings), 1)
        self.assertIn('were not removed: upskill remove ended with state failure', warnings[0])

    def test_an_authentication_failure_is_unavailable(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                return {'state': 'authentication_failure', 'stdout': ''}

        results = q.run_smoke(smoke_args(), FakeRunner(answer))
        self.assertEqual({cell['result'] for cell in results}, {'unavailable'})

    def test_a_failing_version_command_makes_the_client_unavailable(self):
        def answer(command):
            if command[1:] == ['--version']:
                return {'state': 'unavailable', 'stdout': ''}

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(), runner)
        self.assertEqual({cell['result'] for cell in results}, {'unavailable'})
        self.assertEqual({cell['client_version'] for cell in results}, {'unknown'})

    def test_each_kind_passes_only_with_its_own_marker(self):
        seen = {}

        def answer(command):
            if command[1] == 'add':
                source = Path(command[2])
                for kind, path in (('rule', 'rules'), ('skill', 'skills'), ('agent', 'agents')):
                    item = next((source / path).glob('*/*.md'))
                    text = item.read_text()
                    seen[kind] = next(w for w in text.split() if w.startswith('SMOKE_'))
                return {'state': 'success', 'stdout': ''}
            if command[0] == 'claude' and '--version' not in command:
                prompt = command[-1]
                kind = next(k for k in q.SMOKE_KINDS if prompt == q.smoke_prompt(k, prompt.split('`')[1]))
                return {'state': 'success', 'stdout': seen[kind] + '\n'}

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(), runner)
        self.assertEqual({cell['result'] for cell in results}, {'pass'})
        self.assertEqual(len(set(seen.values())), 3)
        for command in runner.commands:
            for marker in seen.values():
                self.assertNotIn(marker, ' '.join(command))
        self.assertEqual({cell['client_version'] for cell in results}, {'1.2.3'})

    def test_a_kind_is_supported_only_when_both_scopes_pass(self):
        cells = [
            {'client': 'claude', 'kind': 'rule', 'scope': 'project', 'result': 'pass', 'client_version': '1.2.3'},
            {'client': 'claude', 'kind': 'rule', 'scope': 'global', 'result': 'pass', 'client_version': '1.2.3'},
            {'client': 'claude', 'kind': 'skill', 'scope': 'project', 'result': 'pass', 'client_version': '1.2.3'},
            {'client': 'claude', 'kind': 'skill', 'scope': 'global', 'result': 'fail', 'client_version': '1.2.3'},
            {'client': 'claude', 'kind': 'agent', 'scope': 'project', 'result': 'pass', 'client_version': '1.2.3'},
        ]
        levels = q.smoke_levels(cells)
        self.assertEqual(levels[('claude', 'rule')], 'supported')
        self.assertEqual(levels[('claude', 'skill')], 'experimental')
        self.assertEqual(levels[('claude', 'agent')], 'experimental')

    def test_exit_code_is_0_when_a_cell_fails(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                return {'state': 'success', 'stdout': 'nothing useful'}

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = q.smoke_main(smoke_args(), FakeRunner(answer))
        self.assertEqual(code, 0)
        lines = out.getvalue().splitlines()
        self.assertIn('claude rule project fail 1.2.3', lines)
        self.assertIn('claude rule experimental', lines)

    def test_copilot_response_is_read_from_the_message_events(self):
        stdout = '{"type":"assistant.message","data":{"content":"MARK"}}\n{"type":"result","exitCode":0}\n'
        self.assertEqual(q.smoke_response('copilot', stdout), 'MARK')
        self.assertEqual(q.smoke_response('claude', ' MARK \n'), 'MARK')

    def test_the_client_command_never_grants_shell_or_file_tools(self):
        for client in q.SMOKE_CLIENTS:
            for kind in q.SMOKE_KINDS:
                joined = ' '.join(q.smoke_command(client, kind, 'prompt', 'project'))
                self.assertNotIn('allow-all', joined)
                self.assertNotIn('yolo', joined)
                self.assertNotIn('Bash', joined)


@unittest.skipUnless(BINARY.exists(), 'target/debug/upskill is not built')
class SmokeSourceInstallTest(unittest.TestCase):
    def test_the_source_installs_the_three_kinds_for_both_clients(self):
        env = q.isolated_git_environment()
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            home = base / 'home'
            repo = base / 'repo'
            home.mkdir()
            repo.mkdir()
            env['HOME'] = str(home)
            env['USERPROFILE'] = str(home)
            subprocess.run(['git', 'init', '-q', str(repo)], env=env, check=True)
            markers = {'rule': 'MARK_R', 'skill': 'MARK_S', 'agent': 'MARK_A'}
            source = q.smoke_source(base / 'source', 'upskill-smoke-test', markers)
            done = subprocess.run([str(BINARY), 'add', str(source), '--claude', '--copilot', '--project'],
                                  cwd=repo, env=env, capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            installed = [p for p in repo.rglob('*') if p.is_file() and '.git' not in p.parts]
            for marker in markers.values():
                holders = [p for p in installed if marker in p.read_text(errors='ignore')]
                self.assertGreaterEqual(len(holders), 2, (marker, holders))
            names = {str(p.relative_to(repo)) for p in installed}
            for expected in ('.claude/rules/upskill-smoke-test.md', '.claude/skills/upskill-smoke-test/SKILL.md',
                             '.claude/agents/upskill-smoke-test.md',
                             '.github/instructions/upskill-smoke-test.instructions.md',
                             '.github/skills/upskill-smoke-test/SKILL.md',
                             '.github/agents/upskill-smoke-test.agent.md'):
                self.assertIn(expected, names)


if __name__ == '__main__':
    unittest.main()
