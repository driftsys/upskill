"""Run a harness-built command with terminal stdin and an optional answer."""

import json
import os
import pty
import select
import signal
import subprocess
import sys
import time


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
    # With "interrupt_at" (which needs "merge"), read the output until it
    # contains that text, then send SIGINT to the process group of the
    # command, then read the rest of the output.
    interrupt_at = spec.get("interrupt_at")
    early = b""
    if interrupt_at:
        assert spec.get("merge"), "interrupt_at needs merge"
        deadline = time.monotonic() + 5
        while interrupt_at.encode("utf-8") not in early:
            remaining = deadline - time.monotonic()
            ready, _, _ = select.select([child.stdout], [], [], max(remaining, 0))
            if not ready:
                break
            chunk = os.read(child.stdout.fileno(), 4096)
            if not chunk:
                break
            early += chunk
        if interrupt_at.encode("utf-8") in early:
            os.killpg(child.pid, signal.SIGINT)
    try:
        stdout, stderr = child.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(child.pid, signal.SIGKILL)
        stdout, stderr = child.communicate(timeout=5)
    stdout = early + stdout
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
