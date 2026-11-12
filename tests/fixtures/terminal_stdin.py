"""Run a harness-built command with terminal stdin and an optional answer."""

import json
import os
import pty
import signal
import subprocess
import sys


spec = json.loads(sys.argv[1])
env = os.environ.copy()
for key, value in spec["env"]:
    if value is None:
        env.pop(key, None)
    else:
        env[key] = value
assert env["HOME"] == spec["home"]
assert env["USERPROFILE"] == spec["home"]

# Verify the child's actual stdin before replacing this process with the CLI.
verify_and_exec = """
import os
import sys
assert os.isatty(0), "child stdin is not a terminal"
sys.stderr.write("terminal stdin verified\\n")
sys.stderr.flush()
os.execv(sys.argv[1], sys.argv[1:])
"""

master, slave = pty.openpty()
child = None
try:
    child = subprocess.Popen(
        [sys.executable, "-c", verify_and_exec, spec["program"], *spec["args"]],
        cwd=spec["cwd"],
        env=env,
        stdin=slave,
        stdout=subprocess.PIPE,
        # With "merge", stderr goes to the same pipe as stdout, so that the
        # order of the two streams is kept.
        stderr=subprocess.STDOUT if spec.get("merge") else subprocess.PIPE,
        start_new_session=True,
    )
    os.close(slave)
    slave = None
    # Write the optional answer. Without one, keep the master open without
    # writing input: a confirmation read must wait.
    if spec.get("input"):
        os.write(master, spec["input"].encode("utf-8"))
    timed_out = False
    try:
        stdout, stderr = child.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(child.pid, signal.SIGKILL)
        stdout, stderr = child.communicate(timeout=5)
    print(json.dumps({
        "timed_out": timed_out,
        "returncode": child.returncode,
        "stdout": stdout.decode("utf-8", errors="replace"),
        "stderr": "" if stderr is None else stderr.decode("utf-8", errors="replace"),
    }))
finally:
    if child is not None and child.poll() is None:
        os.killpg(child.pid, signal.SIGKILL)
        child.wait(timeout=5)
    if slave is not None:
        os.close(slave)
    os.close(master)
