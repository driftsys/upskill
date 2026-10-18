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


class QualificationTests(unittest.TestCase):
    def test_fixture_markers_are_separated(self):
        with tempfile.TemporaryDirectory() as tmp:
            for client in q.CLIENTS:
                root = Path(tmp) / client
                positive, negative, marker, staging_marker = q.fixtures(root, client, False)
                positives = [p for p in positive.rglob('*') if '.git' not in p.relative_to(positive).parts]
                negatives = [p for p in negative.rglob('*') if '.git' not in p.relative_to(negative).parts]
                canonical = Path(q.CLIENTS[client])
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
                result = subprocess.run(['git', '-C', str(fixture), 'rev-parse', '--verify', 'HEAD'], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, 'fixture needs a committed root for native project discovery')
                heads.append(result.stdout.strip())
                status = subprocess.run(['git', '-C', str(fixture), 'ls-tree', '--name-only', 'HEAD'], capture_output=True, text=True, check=True)
                self.assertEqual(status.stdout, '')
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

class OpenCodeEvents(unittest.TestCase):
    def test_text_events_and_tool_rejection(self):
        self.assertEqual(q.opencode_response('{"type":"text","part":{"type":"text","text":"marker"}}'), ('marker', False))
        self.assertEqual(q.opencode_response('{"type":"tool_use","part":{"text":"marker"}}')[1], True)
        self.assertEqual(q.opencode_response('not json'), ('', True))


class OrchestrationTests(unittest.TestCase):
    def controlled_run(self, client='claude', failed=None, tool=False, version_state='success', version_text='v1.2.3', events='valid'):
        with tempfile.TemporaryDirectory() as tmp:
            evidence = Path(tmp) / 'evidence'
            calls = []

            def invocation(command, cwd, timeout, env):
                if '--version' in command:
                    return {'state': version_state, 'stdout': version_text, 'sha256': 'version-hash'}
                calls.append((command, dict(env)))
                if cwd.name == 'positive':
                    marker = re.search(r'QUALIFIED_[a-f0-9]+', (cwd / q.CLIENTS[client]).read_text()).group()
                else:
                    marker = re.search(r'ABSENT_[a-f0-9]+', command[-1]).group()
                for secret_marker in ('QUALIFIED_', 'STAGING_'):
                    self.assertNotIn(secret_marker, command[-1])
                state = failed[1] if failed and cwd.name == failed[0] else 'success'
                if client == 'codex':
                    Path(command[command.index('-o') + 1]).write_text(marker)
                    stdout = '\n'.join([json.dumps({'type': 'item.completed', 'item': {'type': 'command_execution' if tool else 'agent_message', 'text': marker}}), json.dumps({'type': 'turn.completed'})])
                    if events == 'empty':
                        stdout = ''
                    elif events == 'malformed':
                        stdout = 'not json'
                    elif events == 'unknown':
                        stdout += '\n' + json.dumps({'type': 'unrecognized_event'})
                    elif events == 'incomplete':
                        stdout = json.dumps({'type': 'item.started', 'item': {'type': 'agent_message'}})
                elif client == 'opencode':
                    stdout = json.dumps({'type': 'tool_use' if tool else 'text', 'text': marker})
                else:
                    stdout = marker
                return {'state': state, 'stdout': stdout, 'sha256': 'output-hash'}

            with mock.patch.object(q, 'invoke', side_effect=invocation), contextlib.redirect_stdout(io.StringIO()):
                q.run(argparse.Namespace(evidence=evidence, clients=[client], timeout=1))
            return json.loads((evidence / 'summary.json').read_text()), calls

    def test_successful_orchestration_qualifies(self):
        for client in q.CLIENTS:
            with self.subTest(client=client):
                report, _ = self.controlled_run(client)
                self.assertTrue(all(probe['result'] == 'qualified' for probe in report['probes']))

    def test_exact_responses_with_failed_invocation_never_qualify(self):
        for state in ('failure', 'authentication_failure', 'timeout', 'unavailable'):
            for fixture in ('positive', 'negative'):
                with self.subTest(state=state, fixture=fixture):
                    report, _ = self.controlled_run(failed=(fixture, state))
                    self.assertNotEqual(report['probes'][0]['result'], 'qualified')

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

    def test_version_failure_or_missing_semver_never_qualifies(self):
        for state, text in [('success', 'not a version'), ('failure', 'v1.2.3'), ('timeout', ''), ('unavailable', '')]:
            with self.subTest(state=state, text=text):
                report, calls = self.controlled_run(version_state=state, version_text=text)
                self.assertEqual(report['probes'][0]['result'], 'version_unqualified')
                self.assertIsNone(report['probes'][0]['version'])
                self.assertEqual(len(calls), 2)

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
        with mock.patch.dict(os.environ, {'OPENCODE_CONFIG_PROJECT_DISABLE': '1', 'OPENCODE_DISABLE_PROJECT_CONFIG': '1', 'OPENCODE_CONFIG_CONTENT': 'arbitrary-sensitive-config'}):
            report, calls = self.controlled_run('opencode')
        for _, env in calls:
            self.assertEqual(env['OPENCODE_CONFIG_PROJECT_DISABLE'], '0')
            self.assertEqual(env['OPENCODE_DISABLE_PROJECT_CONFIG'], '0')
            self.assertNotIn('OPENCODE_CONFIG_CONTENT', env)
        for probe in report['probes']:
            settings = probe['configuration']['discovery_environment']
            self.assertEqual(settings['OPENCODE_CONFIG_PROJECT_DISABLE'], '0')
            self.assertEqual(settings['OPENCODE_DISABLE_PROJECT_CONFIG'], '0')
            self.assertNotIn('arbitrary-sensitive-config', json.dumps(probe))


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
