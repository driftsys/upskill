import argparse
import contextlib
import importlib.util
import io
import json
import os
import re
import signal
import subprocess
import sys
import time
from unittest import mock
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-native-instructions.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)

CANONICAL_PATHS = {
    'codex': 'AGENTS.md',
    'claude': '.claude/rules/probe.md',
    'copilot': '.github/copilot-instructions.md',
    'opencode': 'AGENTS.md',
}


def fixture_git(project, *args):
    env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    return subprocess.run(['git', '-C', str(project), *args], env=env, check=True, capture_output=True, text=True).stdout


class QualificationTests(unittest.TestCase):
    def test_fixture_markers_are_separated(self):
        with tempfile.TemporaryDirectory() as tmp:
            for client in CANONICAL_PATHS:
                root = Path(tmp) / client
                positive, negative, marker, staging_marker = q.fixtures(root, client, False)
                positives = [p for p in positive.rglob('*') if '.git' not in p.relative_to(positive).parts]
                negatives = [p for p in negative.rglob('*') if '.git' not in p.relative_to(negative).parts]
                canonical = Path(CANONICAL_PATHS[client])
                positive_marked = [p.relative_to(positive) for p in positives if p.is_file() and marker in p.read_text()]
                self.assertEqual(positive_marked, [canonical])
                marked = [p for p in negatives if p.is_file() and staging_marker in p.read_text()]
                self.assertEqual(len(marked), 1)
                self.assertEqual(marked[0].parent, negative / canonical.parent)
                self.assertFalse((negative / canonical).exists())
                self.assertFalse(any(marker in p.read_text() for p in negatives if p.is_file()))
                self.assertFalse(any(staging_marker in p.read_text() for p in positives if p.is_file()))
                self.assertTrue(marked[0].name.startswith('.upskill-stage-'))
                self.assertEqual(marked[0].suffix, '')

    def test_fixture_root_commits_have_distinct_project_identities(self):
        with tempfile.TemporaryDirectory() as tmp:
            positive, negative, _, _ = q.fixtures(Path(tmp), 'opencode')
            heads = []
            for fixture in (positive, negative):
                heads.append(fixture_git(fixture, 'rev-parse', '--verify', 'HEAD').strip())
                self.assertEqual(fixture_git(fixture, 'ls-tree', '--name-only', 'HEAD'), '')
            self.assertNotEqual(*heads)

    def test_fixture_git_cannot_modify_inherited_repository(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            external = root / 'external'
            external.mkdir()
            fixture_git(external, 'init', '-q')
            fixture_git(external, '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: external sentinel')
            sentinel = external / 'sentinel'
            sentinel.write_text('staged sentinel\n')
            fixture_git(external, 'add', 'sentinel')
            sentinel.write_text('worktree sentinel\n')
            before_head = fixture_git(external, 'rev-parse', 'HEAD')
            before_index = (external / '.git/index').read_bytes()
            inherited = {'GIT_DIR': str(external / '.git'), 'GIT_WORK_TREE': str(external), 'GIT_INDEX_FILE': str(external / '.git/index')}
            with mock.patch.dict(os.environ, inherited):
                positive, negative, _, _ = q.fixtures(root / 'fixtures', 'opencode')
                self.assertEqual({key: os.environ[key] for key in inherited}, inherited)
            self.assertEqual(fixture_git(external, 'rev-parse', 'HEAD'), before_head)
            self.assertEqual((external / '.git/index').read_bytes(), before_index)
            self.assertEqual(sentinel.read_text(), 'worktree sentinel\n')
            heads = []
            for project in (positive, negative):
                self.assertTrue((project / '.git').is_dir())
                self.assertEqual(Path(fixture_git(project, 'rev-parse', '--show-toplevel').strip()).resolve(), project.resolve())
                heads.append(fixture_git(project, 'rev-parse', 'HEAD'))
                self.assertEqual(fixture_git(project, 'ls-tree', '--name-only', 'HEAD'), '')
            self.assertNotEqual(*heads)

    def test_distinct_negative_marker(self):
        self.assertEqual(q.classify('positive', 'staging', 'positive', 'absent', 'staging'), 'staging_discovered')

    def test_failure_never_qualifies(self):
        for positive in ('failure', 'timeout', 'authentication_failure', 'unavailable'):
            self.assertEqual(q.classify(positive, 'fallback', 'marker', 'fallback'), 'positive_control_failed')
        for negative in ('failure', 'timeout', 'authentication_failure', 'unavailable'):
            self.assertNotEqual(q.classify('marker', negative, 'marker', 'fallback'), 'qualified')

    def test_exact_responses_required(self):
        self.assertEqual(q.classify('marker', 'fallback', 'marker', 'fallback'), 'qualified')
        self.assertEqual(q.classify('marker', 'marker', 'marker', 'fallback'), 'staging_discovered')
        self.assertEqual(q.classify('prefix marker', 'fallback', 'marker', 'fallback'), 'positive_control_failed')
        self.assertEqual(q.classify('marker', 'other', 'marker', 'fallback'), 'inconclusive')

class CopilotClaudeRulesTests(unittest.TestCase):
    def test_reader_fixture_has_only_the_selected_instruction(self):
        with tempfile.TemporaryDirectory() as tmp:
            rows = q.copilot_claude_rules_fixtures(Path(tmp))
            self.assertEqual([row['label'] for row in rows], ['positive', 'candidate', 'candidate-paths'])
            heads = []
            markers = [row['marker'] for row in rows]
            self.assertEqual(len(set(markers)), 3)
            for row in rows:
                project = row['project']
                files = [p for p in project.rglob('*') if p.is_file() and '.git' not in p.relative_to(project).parts]
                path = '.github/copilot-instructions.md' if row['label'] == 'positive' else '.claude/rules/probe.md'
                self.assertEqual([str(p.relative_to(project)) for p in files], [path])
                text = files[0].read_text()
                self.assertIn(row['marker'], text)
                self.assertNotIn(row['fallback'], text)
                self.assertEqual(text.startswith('---\npaths:'), row['label'] == 'candidate-paths')
                for marker in markers:
                    self.assertEqual(marker in text, marker == row['marker'])
                heads.append(fixture_git(project, 'rev-parse', 'HEAD'))
            self.assertEqual(len(set(heads)), 3)

    def test_reader_classification_requires_successful_positive_and_exact_candidate(self):
        self.assertEqual(q.classify_reader('ok', 'seen', 'ok', 'seen', 'absent'), 'reads')
        self.assertEqual(q.classify_reader('ok', 'absent', 'ok', 'seen', 'absent'), 'does not read')
        for failure in q.FAILURES | {'wrong', 'prefix ok'}:
            self.assertEqual(q.classify_reader(failure, 'absent', 'ok', 'seen', 'absent'), 'positive_control_failed')
        for failure in q.FAILURES:
            self.assertEqual(q.classify_reader('ok', failure, 'ok', 'seen', 'absent'), failure)
        for response in ('prefix seen', 'prefix absent', '', 'ok'):
            self.assertEqual(q.classify_reader('ok', response, 'ok', 'seen', 'absent'), 'inconclusive')


class OpenCodeEvents(unittest.TestCase):
    def test_text_events_and_tool_rejection(self):
        self.assertEqual(q.opencode_response('{"type":"text","part":{"type":"text","text":"marker"}}'), ('marker', False))
        self.assertEqual(q.opencode_response('{"type":"tool_use","part":{"text":"marker"}}')[1], True)
        self.assertEqual(q.opencode_response('not json'), ('', True))


class OrchestrationTests(unittest.TestCase):
    def controlled_run(self, client='claude', failed=None, tool=False, version_state='success', version_text='v1.2.3', events='valid', events_fixture=None, discovery='both'):
        with tempfile.TemporaryDirectory() as tmp:
            evidence = Path(tmp) / 'evidence'
            calls = []

            def invocation(command, cwd, timeout, env):
                is_version = '--version' in command
                calls.append({'command': command, 'env': dict(env), 'fixture': None if is_version else cwd.parent.name + '/' + cwd.name, 'config': json.loads((cwd / 'opencode.json').read_text()) if client == 'opencode' and not is_version else None})
                if '--version' in command:
                    return {'state': version_state, 'stdout': version_text, 'sha256': 'version-hash'}
                if cwd.name == 'positive':
                    marker = re.search(r'QUALIFIED_[a-f0-9]+', (cwd / CANONICAL_PATHS[client]).read_text()).group()
                else:
                    marker = re.search(r'ABSENT_[a-f0-9]+', command[-1]).group()
                for secret_marker in ('QUALIFIED_', 'STAGING_'):
                    self.assertNotIn(secret_marker, command[-1])
                state = failed[1] if failed and cwd.name == failed[0] else 'success'
                current_events = events if events_fixture in (None, cwd.name) else 'valid'
                if client == 'codex':
                    Path(command[command.index('-o') + 1]).write_text(marker)
                    stdout = '\n'.join([json.dumps({'type': 'item.completed', 'item': {'type': 'command_execution' if tool else 'agent_message', 'text': marker}}), json.dumps({'type': 'turn.completed'})])
                    if current_events == 'empty':
                        stdout = ''
                    elif current_events == 'malformed':
                        stdout = 'not json'
                    elif current_events == 'unknown':
                        stdout += '\n' + json.dumps({'type': 'unrecognized_event'})
                    elif current_events == 'incomplete':
                        stdout = json.dumps({'type': 'item.completed', 'item': {'type': 'agent_message', 'text': marker}})
                    elif current_events == 'mismatch':
                        stdout = '\n'.join([json.dumps({'type': 'item.completed', 'item': {'type': 'agent_message', 'text': 'different response'}}), json.dumps({'type': 'turn.completed'})])
                elif client == 'opencode':
                    stdout = 'not json' if current_events == 'malformed' else json.dumps({'type': 'tool_use' if tool else 'text', 'text': marker})
                else:
                    stdout = marker
                return {'state': state, 'stdout': stdout, 'sha256': 'output-hash'}

            with mock.patch.object(q, 'invoke', side_effect=invocation), contextlib.redirect_stdout(io.StringIO()):
                q.run(argparse.Namespace(evidence=evidence, clients=[client], timeout=1, opencode_discovery=discovery))
            return json.loads((evidence / 'summary.json').read_text()), calls

    def test_successful_orchestration_qualifies(self):
        for client in CANONICAL_PATHS:
            with self.subTest(client=client):
                report, _ = self.controlled_run(client)
                self.assertTrue(all(probe['result'] == 'qualified' for probe in report['probes']))

    def test_exact_responses_with_failed_invocation_never_qualify(self):
        for client in CANONICAL_PATHS:
            for state in ('failure', 'authentication_failure', 'timeout', 'unavailable'):
                for fixture in ('positive', 'negative'):
                    for events in ('valid', 'empty', 'malformed'):
                        with self.subTest(client=client, state=state, fixture=fixture, events=events):
                            report, _ = self.controlled_run(client, failed=(fixture, state), events=events, events_fixture=fixture)
                            for probe in report['probes']:
                                self.assertEqual(probe['outcomes'][0 if fixture == 'positive' else 1]['state'], state)
                                self.assertEqual(probe['result'], 'positive_control_failed' if fixture == 'positive' else state)

    def test_exact_responses_with_observed_tools_never_qualify(self):
        for client in ('codex', 'opencode'):
            with self.subTest(client=client):
                report, _ = self.controlled_run(client, tool=True)
                self.assertTrue(all(probe['result'] != 'qualified' for probe in report['probes']))
                self.assertTrue(all(outcome['tools_observed'] for probe in report['probes'] for outcome in probe['outcomes']))

    def test_codex_requires_completed_response_events(self):
        for events in ('empty', 'malformed', 'incomplete', 'unknown'):
            with self.subTest(events=events):
                report, _ = self.controlled_run('codex', events=events)
                self.assertNotEqual(report['probes'][0]['result'], 'qualified')

    def test_codex_requires_completed_turn_for_each_control(self):
        for fixture in ('positive', 'negative'):
            with self.subTest(fixture=fixture):
                report, _ = self.controlled_run('codex', events='incomplete', events_fixture=fixture)
                probe = report['probes'][0]
                outcome = probe['outcomes'][0 if fixture == 'positive' else 1]
                self.assertEqual(outcome['state'], 'failure')
                self.assertFalse(outcome['response_events']['completed_response'])
                self.assertEqual(outcome['response'], probe['marker'] if fixture == 'positive' else probe['fallback'])
                self.assertEqual(probe['result'], 'positive_control_failed' if fixture == 'positive' else 'failure')

    def test_codex_requires_event_and_output_identity_for_each_control(self):
        for fixture in ('positive', 'negative'):
            with self.subTest(fixture=fixture):
                report, _ = self.controlled_run('codex', events='mismatch', events_fixture=fixture)
                probe = report['probes'][0]
                outcome = probe['outcomes'][0 if fixture == 'positive' else 1]
                self.assertEqual(outcome['state'], 'failure')
                self.assertTrue(outcome['response_events']['completed_response'])
                self.assertEqual(outcome['response'], probe['marker'] if fixture == 'positive' else probe['fallback'])
                self.assertEqual(probe['result'], 'positive_control_failed' if fixture == 'positive' else 'failure')

    def test_version_failure_or_missing_semver_never_qualifies(self):
        for state, text in [('success', 'not a version'), ('failure', 'v1.2.3'), ('timeout', ''), ('unavailable', '')]:
            with self.subTest(state=state, text=text):
                report, calls = self.controlled_run(version_state=state, version_text=text)
                self.assertEqual(report['probes'][0]['result'], 'version_unqualified')
                self.assertIsNone(report['probes'][0]['version'])
                self.assertEqual(len(calls), 3)

    def test_native_and_version_environments_exclude_inherited_git_authority(self):
        inherited = {'GIT_DIR': '/unrelated/repository.git', 'GIT_WORK_TREE': '/unrelated/worktree', 'GIT_INDEX_FILE': '/unrelated/index', 'GIT_COMMON_DIR': '/unrelated/common', 'GIT_OBJECT_DIRECTORY': '/unrelated/objects', 'GIT_ALTERNATE_OBJECT_DIRECTORIES': '/unrelated/alternates', 'GIT_CONFIG_COUNT': '1', 'GIT_CONFIG_KEY_0': 'core.worktree', 'GIT_CONFIG_VALUE_0': '/unrelated/config-worktree'}
        # Real fixture Git isolation is covered separately; this boundary test
        # records version and model environments without executing a native CLI.
        with mock.patch.dict(os.environ, inherited), mock.patch.object(q.subprocess, 'run'):
            for client in CANONICAL_PATHS:
                with self.subTest(client=client):
                    report, calls = self.controlled_run(client)
                    for call in calls:
                        self.assertFalse(set(inherited) & call['env'].keys())
                    self.assertEqual({key: os.environ[key] for key in inherited}, inherited)
                    self.assertTrue(all(probe['result'] == 'qualified' for probe in report['probes']))

    def test_opencode_discovery_inventory_and_explicit_selections(self):
        deny = [{'action': '*', 'resource': '*', 'effect': 'deny'}]
        for discovery, modes in [('both', ['default', 'configured broad globs']), ('default', ['default']), ('configured', ['configured broad globs'])]:
            with self.subTest(discovery=discovery):
                report, calls = self.controlled_run('opencode', discovery=discovery)
                self.assertEqual([probe['discovery'] for probe in report['probes']], modes)
                self.assertEqual(len(calls), 1 + 2 * len(modes))
                self.assertEqual(calls[0]['command'], ['opencode', '--version'])
                expected_fixtures = []
                for probe, mode in zip(report['probes'], modes):
                    root = 'opencode-configured' if mode == 'configured broad globs' else 'opencode-default'
                    fixtures = [root + '/positive', root + '/negative']
                    expected_fixtures.extend(fixtures)
                    self.assertEqual([outcome['fixture'] for outcome in probe['outcomes']], fixtures)
                    self.assertEqual(probe['configuration']['instruction_path'], 'AGENTS.md')
                    self.assertEqual(probe['configuration']['instructions_globs'], ['AGENTS.md', '.upskill-stage-*'] if mode == 'configured broad globs' else None)
                    self.assertEqual(probe['result'], 'qualified')
                    self.assertEqual(probe['configuration']['permissions'], deny)
                self.assertEqual([call['fixture'] for call in calls[1:]], expected_fixtures)
                for call in calls[1:]:
                    expected = {'permissions': deny}
                    if call['fixture'].startswith('opencode-configured/'):
                        expected['instructions'] = ['AGENTS.md', '.upskill-stage-*']
                    self.assertEqual(call['config'], expected)

    def test_opencode_v2_fixture_and_controlled_environment(self):
        with tempfile.TemporaryDirectory() as tmp:
            for configured in (False, True):
                positive, negative, _, _ = q.fixtures(Path(tmp) / str(configured), 'opencode', configured)
                for project in (positive, negative):
                    config = json.loads((project / 'opencode.json').read_text())
                    self.assertNotIn('permission', config)
                    deny = [{'action': '*', 'resource': '*', 'effect': 'deny'}]
                    self.assertEqual(config['permissions'], deny)
                    self.assertEqual(config.get('instructions'), ['AGENTS.md', '.upskill-stage-*'] if configured else None)
        overrides = {'OPENCODE_CONFIG_CONTENT': 'inline-sensitive-sentinel', 'OPENCODE_CONFIG': '/file-sensitive-sentinel', 'OPENCODE_CONFIG_DIR': '/directory-sensitive-sentinel'}
        with mock.patch.dict(os.environ, {'OPENCODE_CONFIG_PROJECT_DISABLE': '1', 'OPENCODE_DISABLE_PROJECT_CONFIG': '1', **overrides}):
            report, calls = self.controlled_run('opencode')
            self.assertEqual({key: os.environ[key] for key in overrides}, overrides)
        self.assertEqual(len(calls), 5)
        for call in calls:
            env = call['env']
            self.assertEqual(env['OPENCODE_CONFIG_PROJECT_DISABLE'], '0')
            self.assertEqual(env['OPENCODE_DISABLE_PROJECT_CONFIG'], '0')
            for key in overrides:
                self.assertNotIn(key, env)
        for probe in report['probes']:
            settings = probe['configuration']['discovery_environment']
            self.assertEqual(settings['OPENCODE_CONFIG_PROJECT_DISABLE'], '0')
            self.assertEqual(settings['OPENCODE_DISABLE_PROJECT_CONFIG'], '0')
            self.assertEqual(settings['removed_overrides'], ['OPENCODE_CONFIG_CONTENT', 'OPENCODE_CONFIG', 'OPENCODE_CONFIG_DIR'])
            for value in overrides.values():
                self.assertNotIn(value, json.dumps(probe))


@unittest.skipUnless(hasattr(os, 'killpg'), 'POSIX process groups are required')
class ProcessCleanupTests(unittest.TestCase):
    def test_interrupt_terminates_owned_process_group_and_propagates(self):
        self.check_cleanup(interrupt=True)

    def test_timeout_terminates_owned_process_group(self):
        self.check_cleanup(interrupt=False)

    def check_cleanup(self, interrupt):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pidfile = root / 'pids'
            native = "import os,subprocess,sys,time; child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)']); open(sys.argv[1],'w').write(str(os.getpid())+' '+str(child.pid)); time.sleep(30)"
            wrapper = "import importlib.util,os,sys; from pathlib import Path; spec=importlib.util.spec_from_file_location('q',sys.argv[1]); q=importlib.util.module_from_spec(spec); spec.loader.exec_module(q);\ntry: q.invoke([sys.executable,'-c',sys.argv[2],sys.argv[3]],Path(sys.argv[4]),float(sys.argv[5]),os.environ.copy())\nexcept KeyboardInterrupt: sys.exit(42)"
            env = os.environ.copy()
            env['PYTHONDONTWRITEBYTECODE'] = '1'
            wrapper_process = subprocess.Popen([sys.executable, '-c', wrapper, str(Path(q.__file__)), native, str(pidfile), tmp, '30' if interrupt else '0.3'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            pids = []
            try:
                deadline = time.monotonic() + 5
                while not pidfile.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertTrue(pidfile.exists())
                pids = [int(pid) for pid in pidfile.read_text().split()]
                if interrupt:
                    wrapper_process.send_signal(signal.SIGINT)
                self.assertEqual(wrapper_process.wait(timeout=5), 42 if interrupt else 0)
                for pid in pids:
                    status = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True).stdout.strip()
                    self.assertTrue(not status or status.startswith('Z'), f'owned process {pid} still running: {status}')
            finally:
                for pid in pids:
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                if wrapper_process.poll() is None:
                    wrapper_process.kill()
                wrapper_process.wait()


if __name__ == '__main__':
    unittest.main()
