import importlib.util
import os
from unittest import mock
import signal
import subprocess
import sys
import time
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-native-instructions.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)

SCRIPT = str(Path(q.__file__))


class IsolatedGitEnvironmentTests(unittest.TestCase):
    def test_no_git_variable_reaches_the_environment(self):
        sentinels = {
            'GIT_DIR': '/unrelated/repository.git',
            'GIT_WORK_TREE': '/unrelated/worktree',
            'GIT_INDEX_FILE': '/unrelated/index',
            'GIT_COMMON_DIR': '/unrelated/common',
            'GIT_OBJECT_DIRECTORY': '/unrelated/objects',
            'GIT_CONFIG_COUNT': '1',
            'GIT_CONFIG_KEY_0': 'core.worktree',
            'GIT_CONFIG_VALUE_0': '/unrelated/config-worktree',
            'GIT_UPSKILL_SENTINEL_X': 'synthetic-git-value',
        }
        kept = {'PATH': '/synthetic/bin', 'HOME': '/synthetic/home', 'UPSKILL_TEST_ORDINARY': 'x',
                'GITHUB_TOKEN': 'dummy-token', 'UPSKILL_SENTINEL_KEEP': 'keep'}
        with mock.patch.dict(os.environ, {**sentinels, **kept}, clear=True):
            env = q.isolated_git_environment()
        self.assertEqual([key for key in env if key.startswith('GIT_')], [])
        for key, value in kept.items():
            self.assertEqual(env.get(key), value)


class InvokeStateTests(unittest.TestCase):
    def run_child(self, code, timeout=10):
        with tempfile.TemporaryDirectory() as tmp:
            return q.invoke([sys.executable, '-c', code], Path(tmp), timeout, os.environ.copy())

    def test_a_login_message_with_a_failing_exit_is_an_authentication_failure(self):
        result = self.run_child("import sys; print('Not logged in', file=sys.stderr); sys.exit(1)")
        self.assertEqual(result['state'], 'authentication_failure')

    def test_a_login_message_on_standard_output_only_is_an_authentication_failure(self):
        result = self.run_child("import sys; print('Not logged in'); sys.exit(1)")
        self.assertEqual(result['state'], 'authentication_failure')

    def test_a_failing_exit_without_a_login_message_is_a_failure(self):
        self.assertEqual(self.run_child('import sys; sys.exit(3)')['state'], 'failure')

    def test_a_zero_exit_is_a_success_even_with_a_login_message(self):
        self.assertEqual(self.run_child("print('not logged in')")['state'], 'success')

    def test_a_child_that_outlasts_the_timeout_is_a_timeout(self):
        self.assertEqual(self.run_child('import time; time.sleep(30)', timeout=0.3)['state'], 'timeout')

    def test_a_missing_executable_is_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = q.invoke([str(Path(tmp) / 'no-such-client')], Path(tmp), 5, os.environ.copy())
        self.assertEqual(result['state'], 'unavailable')


class CommandLineTests(unittest.TestCase):
    # test_smoke_native.py pins the refusal of the smoke-only flags without
    # --smoke; the refusals below are pinned only here.
    def run_script(self, *flags):
        done = subprocess.run([sys.executable, '-I', SCRIPT, *flags], capture_output=True, text=True)
        self.assertEqual(done.returncode, 2, done.stderr)
        return done.stderr

    def test_the_script_without_smoke_is_a_usage_error(self):
        self.assertIn('--smoke is required', self.run_script())

    def test_a_client_that_is_not_a_smoke_client_is_refused(self):
        for client in ('codex', 'opencode'):
            self.assertIn('invalid choice', self.run_script('--smoke', '--upskill', 'x', '--clients', client))

    def test_smoke_without_the_upskill_flag_is_refused(self):
        self.assertIn('--upskill is required with --smoke', self.run_script('--smoke'))

    def test_run_with_a_missing_upskill_file_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            missing = str(Path(tmp) / 'no-such-upskill')
            self.assertIn('--upskill does not name a file', self.run_script('--smoke', '--run', '--upskill', missing))


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
