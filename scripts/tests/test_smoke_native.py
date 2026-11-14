import argparse
import contextlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
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


def source_markers(source):
    """The marker that each item of an installed source holds, by kind."""
    source = Path(source)
    markers = {}
    for kind, folder in (('rule', 'rules'), ('skill', 'skills'), ('agent', 'agents')):
        text = next((source / folder).glob('*/*.md')).read_text()
        markers[kind] = next(w for w in text.split() if w.startswith('SMOKE_'))
    return markers


class FakeRunner:
    """Records every command with its cwd and env. `answer(command)` returns the scripted output."""

    def __init__(self, answer=None):
        self.commands = []
        self.cwds = []
        self.envs = []
        self.timeouts = []
        self.git_repositories = []
        self.answer = answer

    def __call__(self, command, cwd, timeout, env):
        self.commands.append([str(part) for part in command])
        self.cwds.append(Path(cwd))
        self.envs.append(env)
        self.timeouts.append(timeout)
        self.git_repositories.append((Path(cwd) / '.git').is_dir())
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
    def setUp(self):
        # A global cell reads and restores the lockfile of the home directory,
        # so no test of this class may see the real home directory.
        home = tempfile.TemporaryDirectory()
        self.addCleanup(home.cleanup)
        patcher = mock.patch.dict(os.environ, {'HOME': home.name})
        patcher.start()
        self.addCleanup(patcher.stop)

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
        # The real `invoke` returns the state `timeout`; it never raises.
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                return {'state': 'timeout', 'response': '', 'sha256': None}

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual({cell['result'] for cell in results}, {'timeout'})
        self.assert_the_asked_item_is_removed(runner)

    def test_a_raised_timeout_is_a_timeout_and_the_items_are_removed(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                raise subprocess.TimeoutExpired(command, 5)

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual({cell['result'] for cell in results}, {'timeout'})
        self.assert_the_asked_item_is_removed(runner)

    def assert_the_asked_item_is_removed(self, runner):
        add = next(c for c in runner.commands if c[1] == 'add')
        self.assertIn('--global', add)
        last = runner.commands[-1]
        self.assertEqual(last[1:4], ['remove', '--global', '--yes'])
        prompts = [c[-1] for c in runner.commands if c[0] == 'claude' and '--version' not in c]
        self.assertEqual(len(prompts), 3)
        for prompt in prompts:
            self.assertIn('`' + last[4] + '`', prompt)

    def test_global_items_are_removed_after_an_exception(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                raise RuntimeError('boom')

        runner = FakeRunner(answer)
        with self.assertRaises(RuntimeError):
            q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual(runner.commands[-1][1:4], ['remove', '--global', '--yes'])

    def test_the_removed_name_is_the_installed_name(self):
        installed = []

        def answer(command):
            if command[1] == 'add':
                installed.extend(p.name for folder in ('rules', 'skills', 'agents') for p in (Path(command[2]) / folder).iterdir())

        runner = FakeRunner(answer)
        q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        removed = runner.commands[-1][4]
        self.assertEqual(installed, [removed] * 3)

    def failed_add_then(self, removal):
        """Runs a global cell whose `add` fails and whose `remove` returns `removal`."""
        self.installed_names = []

        def answer(command):
            if command[1] == 'add':
                self.installed_names.append(next((Path(command[2]) / 'rules').iterdir()).name)
                return {'state': 'failure', 'stdout': ''}
            if command[1] == 'remove':
                return removal

        runner = FakeRunner(answer)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), runner)
        removes = [c for c in runner.commands if c[1] == 'remove']
        self.assertEqual(len(removes), 1)
        self.assertEqual(out.getvalue().count('install-failed'), 3)
        return runner, removes[0], out.getvalue()

    def test_items_left_by_a_failed_global_add_are_removed_without_a_warning(self):
        runner, remove, output = self.failed_add_then({'state': 'success', 'stdout': ''})
        self.assertEqual(remove[1:4], ['remove', '--global', '--yes'])
        self.assertEqual([remove[4]], self.installed_names)
        self.assertNotIn('warning:', output)

    def test_a_name_that_is_not_installed_is_not_a_removal_failure(self):
        removal = {'state': 'failure', 'stdout': '', 'stderr': 'error: not in lockfile: NAME\n', 'exit_code': 1}
        names = []

        def answer(command):
            if command[1] == 'add':
                names.append(next((Path(command[2]) / 'rules').iterdir()).name)
                return {'state': 'failure', 'stdout': ''}
            if command[1] == 'remove':
                return dict(removal, stderr=removal['stderr'].replace('NAME', names[0]))

        out = io.StringIO()
        runner = FakeRunner(answer)
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), runner)
        removes = [c for c in runner.commands if c[1] == 'remove']
        self.assertEqual(removes, [[removes[0][0], 'remove', '--global', '--yes', names[0]]])
        self.assertNotIn('warning:', out.getvalue())

    def test_another_removal_failure_after_a_failed_add_is_a_warning(self):
        for stderr in ('error: permission denied\n', 'error: not in lockfile: some-other-name\n', ''):
            _, _, output = self.failed_add_then({'state': 'failure', 'stdout': '', 'stderr': stderr, 'exit_code': 1})
            self.assertEqual(len([l for l in output.splitlines() if l.startswith('warning: global items upskill-smoke-')]), 1, stderr)
            self.assertIn('were not removed: upskill remove ended with state failure', output)

    def test_items_are_removed_after_a_timed_out_global_add(self):
        def answer(command):
            if command[1] == 'add':
                return {'state': 'timeout', 'response': '', 'sha256': None}

        runner = FakeRunner(answer)
        results = q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True), runner)
        self.assertEqual({cell['result'] for cell in results}, {'install-failed'})
        self.assertEqual(runner.commands[-1][1:4], ['remove', '--global', '--yes'])
        self.assertFalse([c for c in runner.commands if c[0] == 'claude' and '--version' not in c])

    def test_the_timeout_applies_to_add_remove_and_the_client_but_not_to_the_version_call(self):
        runner = FakeRunner()
        q.run_smoke(smoke_args(scopes=['global'], allow_real_home=True, timeout=7), runner)
        for command, timeout in zip(runner.commands, runner.timeouts):
            expected = 15 if command[1:] == ['--version'] else 7
            self.assertEqual(timeout, expected, command)

    def test_a_relative_binary_path_is_made_absolute(self):
        runner = FakeRunner()
        q.run_smoke(smoke_args(upskill=Path('target/release/upskill'), scopes=['global'], allow_real_home=True), runner)
        absolute = str(Path('target/release/upskill').resolve())
        upskill_commands = [c for c in runner.commands if c[1] in ('add', 'remove')]
        self.assertEqual([c[1] for c in upskill_commands], ['add', 'remove'])
        for command in upskill_commands:
            self.assertEqual(command[0], absolute)

    def test_each_client_installs_for_itself_and_is_asked_in_the_cell_repository(self):
        runner = FakeRunner()
        q.run_smoke(smoke_args(clients=['claude', 'copilot']), runner)
        for client in ('claude', 'copilot'):
            indexes = [i for i, c in enumerate(runner.commands) if c[0] == client and '--version' not in c]
            self.assertEqual(len(indexes), 3)
            add = next(i for i, c in enumerate(runner.commands) if c[1] == 'add' and '--' + client in c)
            others = [c for c in ('--claude', '--copilot') if c != '--' + client]
            self.assertFalse([o for o in others if o in runner.commands[add]])
            self.assertEqual(runner.cwds[add].name, client + '-project')
            self.assertTrue(runner.git_repositories[add])
            for i in indexes:
                self.assertEqual(runner.cwds[i], runner.cwds[add])
                self.assertTrue(runner.git_repositories[i])

    def test_commands_run_without_repository_selectors_or_the_claude_nesting_variable(self):
        runner = FakeRunner()
        with mock.patch.dict(os.environ, {'GIT_DIR': '/nowhere', 'GIT_WORK_TREE': '/nowhere', 'GIT_CONFIG_COUNT': '1', 'CLAUDECODE': '1', 'KEEP_ME': 'x'}):
            q.run_smoke(smoke_args(), runner)
        self.assertTrue(runner.envs)
        for env in runner.envs:
            self.assertEqual([k for k in env if k.startswith('GIT_')], [])
            self.assertEqual(env.get('KEEP_ME'), 'x')
            self.assertNotIn('CLAUDECODE', env)

    def test_the_markers_and_the_name_differ_between_runs(self):
        def run_once():
            seen = {}

            def answer(command):
                if command[1] == 'add':
                    seen['markers'] = sorted(source_markers(command[2]).values())
                    seen['name'] = next((Path(command[2]) / 'rules').iterdir()).name

            q.run_smoke(smoke_args(), FakeRunner(answer))
            return seen

        first, second = run_once(), run_once()
        self.assertEqual(len(set(first['markers'])), 3)
        self.assertFalse(set(first['markers']) & set(second['markers']))
        self.assertNotEqual(first['name'], second['name'])

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

    def test_a_removal_that_times_out_is_one_warning_naming_the_timeout(self):
        def answer(command):
            if command[1] == 'remove':
                return {'state': 'timeout', 'stdout': ''}

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), FakeRunner(answer))
        warnings = [line for line in out.getvalue().splitlines() if line.startswith('warning:')]
        self.assertEqual(len(warnings), 1)
        self.assertIn('were not removed: upskill remove ended with state timeout', warnings[0])

    def test_a_raised_timeout_of_the_removal_is_a_warning(self):
        def answer(command):
            if command[1] == 'remove':
                raise subprocess.TimeoutExpired(command, 5)

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), FakeRunner(answer))
        warnings = [line for line in out.getvalue().splitlines() if line.startswith('warning:')]
        self.assertEqual(len(warnings), 1)
        self.assertIn('upskill remove timed out', warnings[0])

    def test_a_project_cell_never_removes_and_never_uses_the_global_scope(self):
        for add_state in ('success', 'failure', 'timeout'):
            def answer(command, add_state=add_state):
                if command[1] == 'add':
                    return {'state': add_state, 'stdout': ''}

            runner = FakeRunner(answer)
            q.run_smoke(smoke_args(clients=['claude', 'copilot'], scopes=['project']), runner)
            self.assertTrue([c for c in runner.commands if c[1] == 'add'])
            self.assertFalse([c for c in runner.commands if c[1] == 'remove'], add_state)
            self.assertFalse([c for c in runner.commands if '--global' in c], add_state)

    def test_the_real_invoke_returns_the_standard_error_text(self):
        code = "import sys; sys.stderr.write('error: not in lockfile: X\\n'); sys.exit(1)"
        with tempfile.TemporaryDirectory() as tmp:
            result = q.invoke([sys.executable, '-c', code], Path(tmp), 10, os.environ.copy())
        self.assertEqual(result['state'], 'failure')
        self.assertEqual(result['exit_code'], 1)
        self.assertEqual(result['stderr'], 'error: not in lockfile: X\n')

    def test_an_authentication_failure_is_unavailable(self):
        def answer(command):
            if command[0] == 'claude' and '--version' not in command:
                return {'state': 'authentication_failure', 'stdout': ''}

        results = q.run_smoke(smoke_args(), FakeRunner(answer))
        self.assertEqual({cell['result'] for cell in results}, {'unavailable'})

    def test_an_exceeded_copilot_quota_is_unavailable(self):
        # Event shapes observed with Copilot CLI 1.0.110 when the monthly quota
        # is used up: the model call fails with status 402, a `session.error`
        # event follows, and the run ends with a `result` event of exit code 1.
        stream = (
            '{"type":"session.start","data":{"sessionId":"S","copilotVersion":"1.0.110"}}\n'
            '{"type":"user.message","data":{"content":"question"}}\n'
            '{"type":"model.call_failure","data":{"statusCode":402}}\n'
            '{"type":"session.error","data":{"errorType":"quota","message":"You have exceeded your monthly quota","statusCode":402}}\n'
            '{"type":"assistant.idle","data":{},"ephemeral":true}\n'
            '{"type":"result","exitCode":1}\n')

        def answer(command):
            if command[0] == 'copilot' and '--version' not in command:
                return {'state': 'failure', 'stdout': stream}

        results = q.run_smoke(smoke_args(clients=['copilot']), FakeRunner(answer))
        self.assertEqual({cell['result'] for cell in results}, {'unavailable'})

    def test_another_copilot_session_error_is_a_failure(self):
        stream = ('{"type":"session.error","data":{"errorType":"query","message":"Model is not available."}}\n'
                  '{"type":"result","exitCode":1}\n')

        def answer(command):
            if command[0] == 'copilot' and '--version' not in command:
                return {'state': 'failure', 'stdout': stream}

        results = q.run_smoke(smoke_args(clients=['copilot'], kinds=['rule']), FakeRunner(answer))
        self.assertEqual([cell['result'] for cell in results], ['fail'])

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
        def cell(kind, scope, result):
            return {'client': 'claude', 'kind': kind, 'scope': scope, 'result': result, 'client_version': '1.2.3'}

        cells = [
            cell('rule', 'project', 'pass'), cell('rule', 'global', 'pass'),
            cell('skill', 'project', 'pass'), cell('skill', 'global', 'fail'),
            cell('agent', 'project', 'pass'),
            cell('hook', 'global', 'pass'), cell('hook', 'project', 'fail'),
            cell('mcp', 'global', 'pass'),
        ]
        levels = q.smoke_levels(cells)
        self.assertEqual(levels[('claude', 'rule')], 'supported')
        self.assertEqual(levels[('claude', 'skill')], 'experimental')
        self.assertEqual(levels[('claude', 'agent')], 'experimental')
        self.assertEqual(levels[('claude', 'hook')], 'experimental')
        self.assertEqual(levels[('claude', 'mcp')], 'experimental')

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

    def test_a_copilot_stream_with_tool_events_is_a_complete_run(self):
        # Event shapes as recorded in test_replay_copilot_agent.py.
        events = [
            {'type': 'tool.execution_start', 'data': {'toolCallId': 'call-1', 'toolName': 'skill', 'arguments': {'skill': 's'}}},
            {'type': 'tool.execution_complete', 'data': {'toolCallId': 'call-1', 'success': True}},
            {'type': 'assistant.message', 'data': {'content': 'MARK'}},
            {'type': 'result', 'exitCode': 0},
        ]
        stdout = ''.join(json.dumps(event) + '\n' for event in events)
        self.assertEqual(q.smoke_response('copilot', stdout), 'MARK')
        seen = {}

        def answer(command):
            if command[1] == 'add':
                seen['skill'] = source_markers(command[2])['skill']
            if command[0] == 'copilot' and '--version' not in command:
                done = [dict(e) for e in events]
                done[2] = {'type': 'assistant.message', 'data': {'content': seen['skill']}}
                return {'state': 'success', 'stdout': ''.join(json.dumps(e) + '\n' for e in done)}

        results = q.run_smoke(smoke_args(clients=['copilot'], kinds=['skill']), FakeRunner(answer))
        self.assertEqual([cell['result'] for cell in results], ['pass'])

    def test_a_copilot_stream_that_is_not_complete_has_no_response(self):
        message = '{"type":"assistant.message","data":{"content":"MARK"}}\n'
        for stdout in (message,
                       message + '{"type":"result","exitCode":1}\n',
                       message + '{"type":"result"}\n',
                       message + '{"type":"result","exitCode":false}\n',
                       message + '{"type":"result","exitCode":0}\n{"type":"assistant.idle"}\n',
                       message + '{"type":"result","exitCode":0}\n{"type":"result","exitCode":0}\n',
                       ''):
            self.assertIsNone(q.smoke_response('copilot', stdout), stdout)

    def test_a_copilot_cell_with_the_marker_but_an_incomplete_stream_fails(self):
        seen = {}

        def answer(command):
            if command[1] == 'add':
                seen['rule'] = source_markers(command[2])['rule']
            if command[0] == 'copilot' and '--version' not in command:
                return {'state': 'success', 'stdout': '{"type":"assistant.message","data":{"content":"' + seen['rule'] + '"}}\n' + self.tail}

        for tail, expected in (('{"type":"result","exitCode":0}\n', 'pass'), ('', 'fail'), ('{"type":"result","exitCode":1}\n', 'fail')):
            self.tail = tail
            results = q.run_smoke(smoke_args(clients=['copilot'], kinds=['rule']), FakeRunner(answer))
            self.assertEqual([cell['result'] for cell in results], [expected], tail)

    def test_the_client_command_is_exactly_the_restricted_command(self):
        mcp = '{"mcpServers":{}}'
        expected = {
            ('claude', 'rule', 'project'): ['claude', '-p', '--tools', '', '--no-session-persistence', '--strict-mcp-config', '--mcp-config', mcp, '--setting-sources', 'project', 'P'],
            ('claude', 'skill', 'project'): ['claude', '-p', '--tools', 'Skill', '--no-session-persistence', '--strict-mcp-config', '--mcp-config', mcp, '--setting-sources', 'project', 'P'],
            ('claude', 'agent', 'global'): ['claude', '-p', '--tools', 'Agent', '--no-session-persistence', '--strict-mcp-config', '--mcp-config', mcp, '--setting-sources', 'user', 'P'],
            ('copilot', 'rule', 'project'): ['copilot', '--no-auto-update', '--disable-builtin-mcps', '--available-tools=', '--no-ask-user', '--output-format', 'json', '--log-level', 'none', '-p', 'P'],
            ('copilot', 'skill', 'global'): ['copilot', '--no-auto-update', '--disable-builtin-mcps', '--available-tools=skill', '--allow-tool=skill', '--no-ask-user', '--output-format', 'json', '--log-level', 'none', '-p', 'P'],
            ('copilot', 'agent', 'project'): ['copilot', '--no-auto-update', '--disable-builtin-mcps', '--available-tools=task', '--allow-tool=task', '--no-ask-user', '--output-format', 'json', '--log-level', 'none', '-p', 'P'],
        }
        for (client, kind, scope), command in expected.items():
            self.assertEqual(q.smoke_command(client, kind, 'P', scope), command)

    def test_smoke_flags_are_refused_without_smoke(self):
        # The parser refuses before any client or upskill command can start.
        script = str(ROOT / 'scripts' / 'qualify-native-instructions.py')
        for flags in (['--kinds', 'rule'], ['--scopes', 'project'], ['--allow-real-home'], ['--upskill', 'x']):
            done = subprocess.run([sys.executable, '-I', script, *flags], capture_output=True, text=True)
            self.assertEqual(done.returncode, 2, flags)
            self.assertIn('only apply with --smoke', done.stderr)

    def test_the_defaults_cover_both_clients_all_kinds_and_both_scopes(self):
        # Without --run the script starts no client and no upskill command.
        script = str(ROOT / 'scripts' / 'qualify-native-instructions.py')
        done = subprocess.run([sys.executable, '-I', script, '--smoke', '--upskill', '/nonexistent/upskill'],
                              capture_output=True, text=True)
        self.assertEqual(done.returncode, 0, done.stderr)
        expected = ['%s %s %s not-run -' % (client, kind, scope)
                    for client in ('claude', 'copilot') for scope in ('project', 'global') for kind in ('rule', 'skill', 'agent')]
        self.assertEqual(done.stdout.splitlines(), expected)

    def test_without_run_the_output_lists_the_cells_and_no_level(self):
        out = io.StringIO()
        runner = FakeRunner()
        with contextlib.redirect_stdout(out):
            code = q.smoke_main(smoke_args(run=False), runner)
        self.assertEqual(code, 0)
        self.assertEqual(runner.commands, [])
        lines = out.getvalue().splitlines()
        self.assertEqual(lines, ['claude %s project not-run -' % kind for kind in q.SMOKE_KINDS])


@unittest.skipUnless(BINARY.exists(), 'target/debug/upskill is not built')
class SmokeGlobalLockfileTest(unittest.TestCase):
    """A global cell leaves `.upskill-lock.json` and `.upskill-lock.json.v1` of
    the home directory as it found them."""

    ORIGINAL = b'{"schema": 1, "items": []}\n'
    ADOPTED = b'{"schema": 2, "items": []}\n'

    def setUp(self):
        home = tempfile.TemporaryDirectory()
        self.addCleanup(home.cleanup)
        self.home = Path(home.name)
        self.lock = self.home / '.upskill-lock.json'
        self.v1 = self.home / '.upskill-lock.json.v1'
        patcher = mock.patch.dict(os.environ, {'HOME': home.name})
        patcher.start()
        self.addCleanup(patcher.stop)

    def run_cell(self, add_state='success', removal=None):
        """The fake `add` adopts the lockfile to schema 2 and writes the copy `.v1`."""
        removal = removal or {'state': 'success', 'stdout': ''}

        def answer(command):
            if command[1] == 'add':
                self.lock.write_bytes(self.ADOPTED)
                self.v1.write_bytes(self.ORIGINAL)
                return {'state': add_state, 'stdout': ''}
            if command[1] == 'remove':
                return removal

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), FakeRunner(answer))
        return out.getvalue()

    def test_the_lockfile_is_restored_after_a_successful_removal(self):
        self.lock.write_bytes(self.ORIGINAL)
        output = self.run_cell()
        self.assertEqual(self.lock.read_bytes(), self.ORIGINAL)
        self.assertFalse(self.v1.exists())
        self.assertNotIn('warning:', output)

    def test_the_lockfile_is_restored_after_a_failed_add_and_a_name_that_is_not_installed(self):
        self.lock.write_bytes(self.ORIGINAL)
        removal = {'state': 'failure', 'stdout': '', 'stderr': 'error: not in lockfile: x\n', 'exit_code': 1}

        def answer(command):
            if command[1] == 'add':
                self.lock.write_bytes(self.ADOPTED)
                self.v1.write_bytes(self.ORIGINAL)
                self.name = next((Path(command[2]) / 'rules').iterdir()).name
                return {'state': 'failure', 'stdout': ''}
            if command[1] == 'remove':
                return dict(removal, stderr='error: not in lockfile: ' + self.name + '\n')

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            q.smoke_main(smoke_args(scopes=['global'], allow_real_home=True), FakeRunner(answer))
        self.assertEqual(self.lock.read_bytes(), self.ORIGINAL)
        self.assertFalse(self.v1.exists())
        self.assertNotIn('warning:', out.getvalue())

    def test_the_lockfile_is_not_restored_after_a_failed_removal(self):
        self.lock.write_bytes(self.ORIGINAL)
        output = self.run_cell(removal={'state': 'failure', 'stdout': '', 'stderr': 'error: permission denied\n', 'exit_code': 1})
        self.assertEqual(self.lock.read_bytes(), self.ADOPTED)
        self.assertTrue(self.v1.exists())
        warnings = [line for line in output.splitlines() if line.startswith('warning:')]
        self.assertEqual(len(warnings), 1)
        self.assertIn('were not removed: upskill remove ended with state failure', warnings[0])
        self.assertIn('lockfile was left as the run changed it', warnings[0])

    def test_a_lockfile_that_did_not_exist_is_deleted(self):
        output = self.run_cell()
        self.assertFalse(self.lock.exists())
        self.assertFalse(self.v1.exists())
        self.assertNotIn('warning:', output)

    def test_a_v1_copy_that_existed_before_is_kept(self):
        self.lock.write_bytes(self.ORIGINAL)
        self.v1.write_bytes(b'older copy\n')
        self.run_cell()
        self.assertEqual(self.lock.read_bytes(), self.ORIGINAL)
        self.assertTrue(self.v1.exists())

    def test_an_error_while_restoring_is_a_warning_and_does_not_raise(self):
        self.lock.write_bytes(self.ORIGINAL)
        with mock.patch.object(q, 'restore_global_lockfile', side_effect=OSError('disk full'), create=True):
            output = self.run_cell()
        self.assertIn('warning: ', output)
        self.assertIn('disk full', output)


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
            installed = {str(p.relative_to(repo)): p.read_text(errors='ignore')
                         for p in repo.rglob('*') if p.is_file() and '.git' not in p.parts}
            n = 'upskill-smoke-test'
            expected = {
                '.claude/rules/' + n + '.md': 'rule', '.github/instructions/' + n + '.instructions.md': 'rule',
                '.claude/skills/' + n + '/SKILL.md': 'skill', '.github/skills/' + n + '/SKILL.md': 'skill',
                '.claude/agents/' + n + '.md': 'agent', '.github/agents/' + n + '.agent.md': 'agent',
            }
            for path, kind in expected.items():
                self.assertIn(path, installed)
                self.assertIn(markers[kind], installed[path], path)
                for other, marker in markers.items():
                    if other != kind:
                        self.assertNotIn(marker, installed[path], path)


if __name__ == '__main__':
    unittest.main()
