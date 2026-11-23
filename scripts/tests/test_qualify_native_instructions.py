import importlib.util
import os
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
