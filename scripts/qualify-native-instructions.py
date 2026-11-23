#!/usr/bin/env python3
"""Smoke test of installed rules, skills and agents against the real AI
clients; no credentials are read or copied. See `just smoke`.

Without --run, the script lists the cells and starts no client; --run makes
paid model calls. Without --smoke, the script exits with a usage error."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import re
import signal
import subprocess
import tempfile


def isolated_git_environment():
    # Repository selectors and injected Git configuration must not redirect
    # disposable fixtures or the clients' project discovery.
    return {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}


def invoke(command, cwd, timeout, env):
    try:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            return {'state': 'timeout', 'response': '', 'sha256': None}
        finally:
            # The child owns this new session. Reap it and stop any surviving
            # descendants before returning or propagating cancellation.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.communicate()
        result = subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
    except FileNotFoundError:
        return {'state': 'unavailable', 'response': '', 'sha256': None}
    except subprocess.TimeoutExpired:
        return {'state': 'timeout', 'response': '', 'sha256': None}
    output = result.stdout + result.stderr
    digest = hashlib.sha256(output.encode()).hexdigest()
    lower = output.lower()
    state = 'success' if result.returncode == 0 else 'failure'
    if state != 'success' and any(term in lower for term in ('not logged in', 'authentication', 'unauthorized', 'login required', 'please log in')):
        state = 'authentication_failure'
    # The result holds the raw stdout and stderr of the client, a hash of them
    # and a diagnostic category. The smoke code prints only the cell result.
    reason = next((label for label, terms in {'permission_denied': ('operation not permitted', 'permission denied'), 'network_failure': ('connection refused', 'failed to connect', 'network', 'dns'), 'authentication_failure': ('not logged in', 'authentication', 'unauthorized', 'login required'), 'configuration_error': ('invalid', 'unknown option', 'unexpected argument')}.items() if any(term in lower for term in terms)), None)
    return {'state': state, 'stdout': result.stdout, 'stderr': result.stderr, 'sha256': digest, 'exit_code': result.returncode, 'diagnostic_category': reason}


SMOKE_CLIENTS = ('claude', 'copilot')
SMOKE_KINDS = ('rule', 'skill', 'agent')
SMOKE_SCOPES = ('project', 'global')
SMOKE_UNAVAILABLE = ('unavailable', 'authentication_failure')
# Tool names that a client needs to load a skill or to delegate to an agent.
# A rule needs no tool. Shell and file tools are never made available.
SMOKE_TOOLS = {
    'claude': {'rule': '', 'skill': 'Skill', 'agent': 'Agent'},
    'copilot': {'rule': '', 'skill': 'skill', 'agent': 'task'},
}


def smoke_source(root, name, marker):
    """Write one rule, one skill and one agent named `name` under
    root/rules, root/skills, root/agents. Each body tells the client to
    answer the smoke question for its kind with `marker[kind]`.
    Returns the source path."""
    root = Path(root)
    entries = {
        'rule': ('rules', 'RULE.md', 'Use when asked the smoke rule question for ' + name,
                 'When you are asked the smoke rule question for `' + name + '`, reply with exactly ' + marker['rule'] + ' and nothing else.'),
        'skill': ('skills', 'SKILL.md', 'Use when asked to use the skill ' + name,
                  'This skill has one step: reply with exactly ' + marker['skill'] + ' and nothing else.'),
        'agent': ('agents', 'AGENT.md', 'Use when asked to delegate to the agent ' + name,
                  'Your whole reply is exactly ' + marker['agent'] + ' and nothing else.'),
    }
    for folder, filename, description, body in entries.values():
        item = root / folder / name
        item.mkdir(parents=True)
        (item / filename).write_text('---\nschema: 1\nname: ' + name + '\ndescription: ' + description + '\n---\n\n' + body + '\n')
    return root


def smoke_prompt(kind, name):
    """The question for one cell. rule: asks for the marker of the standing
    instructions. skill: asks the client to use the skill `name`.
    agent: asks the client to delegate to the agent `name`."""
    if kind == 'rule':
        return ('Answer the smoke rule question for `' + name + '` from your standing instructions. '
                'Reply with only the text that those instructions require. If you have no such instruction, reply NONE.')
    if kind == 'skill':
        return 'Use the skill `' + name + '` and reply with only the text that it requires.'
    return 'Delegate to the agent `' + name + '` and reply with only the text that it returns. Do not answer yourself.'


def classify_smoke(response, marker, other_markers):
    """'pass' when the response holds `marker` and none of `other_markers`;
    'fail' otherwise."""
    if marker in response and not any(other in response for other in other_markers):
        return 'pass'
    return 'fail'


def smoke_command(client, kind, prompt, scope):
    """The client command for one cell: no model memory of earlier sessions,
    no MCP servers, and only the tool that the kind needs."""
    tools = SMOKE_TOOLS[client][kind]
    if client == 'claude':
        sources = 'project' if scope == 'project' else 'user'
        return ['claude', '-p', '--tools', tools, '--no-session-persistence', '--strict-mcp-config',
                '--mcp-config', '{"mcpServers":{}}', '--setting-sources', sources, prompt]
    allowed = ['--available-tools=' + tools] + (['--allow-tool=' + tools] if tools else [])
    return ['copilot', '--no-auto-update', '--disable-builtin-mcps', *allowed, '--no-ask-user',
            '--output-format', 'json', '--log-level', 'none', '-p', prompt]


def smoke_response(client, stdout):
    """The text that the client answered, or None when the run is not complete.
    A Copilot run is complete when its stream has exactly one `result` event,
    that event is the last one, and its exit code is 0. The function does not
    reject tool events, several messages or unknown event types: a skill or
    agent cell uses a tool and may answer in several messages."""
    if client != 'copilot':
        return stdout.strip()
    texts = []
    kinds = []
    results = []
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue
        kinds.append(event.get('type'))
        if event.get('type') == 'result':
            results.append(event)
        if event.get('type') == 'assistant.message' and isinstance(event.get('data'), dict):
            content = event['data'].get('content')
            if isinstance(content, str):
                texts.append(content)
    if (len(results) != 1 or kinds[-1] != 'result'
            or type(results[0].get('exitCode')) is not int or results[0]['exitCode'] != 0):
        return None
    return ''.join(texts).strip()


def smoke_quota_exceeded(client, stdout):
    """True when a Copilot stream holds a `session.error` event of type
    `quota`: the account has no model calls left, so the cell says nothing
    about the client."""
    if client != 'copilot':
        return False
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (isinstance(event, dict) and event.get('type') == 'session.error'
                and isinstance(event.get('data'), dict) and event['data'].get('errorType') == 'quota'):
            return True
    return False


def smoke_levels(cells):
    """The level that the support table may hold for each client and kind:
    'supported' when both scopes passed, 'experimental' otherwise."""
    levels = {}
    for cell in cells:
        key = (cell['client'], cell['kind'])
        passed = {c['scope'] for c in cells if (c['client'], c['kind']) == key and c['result'] == 'pass'}
        levels[key] = 'supported' if set(SMOKE_SCOPES) <= passed else 'experimental'
    return levels


def smoke_cell_result(client, kind, scope, cwd, markers, name, args, runner, env):
    """The pair (result, reason) of one cell; the reason is None when the result
    needs none."""
    prompt = smoke_prompt(kind, name)
    cmd = smoke_command(client, kind, prompt, scope)
    try:
        result = runner(cmd, cwd, args.timeout, env)
    except subprocess.TimeoutExpired:
        return 'timeout', None
    if result['state'] in SMOKE_UNAVAILABLE:
        return 'unavailable', None
    if result['state'] == 'timeout':
        return 'timeout', None
    if smoke_quota_exceeded(client, result.get('stdout', '')):
        return 'unavailable', client + ' reports that the monthly quota is exceeded'
    if result['state'] != 'success':
        return 'fail', None
    response = smoke_response(client, result.get('stdout', ''))
    if response is None:
        # A Copilot run without a successful terminal `result` event has no
        # response; the cell fails.
        return 'fail', None
    others = [value for other, value in markers.items() if other != kind]
    return classify_smoke(response, markers[kind], others), None


def smoke_client_version(client, base, runner, env):
    """The version number in the first line of `<client> --version`, or None."""
    found = runner([client, '--version'], base, 15, env)
    text = found.get('stdout', '').strip().splitlines() if found['state'] == 'success' else []
    match = re.search(r'(?<!\d)\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?', text[0]) if text else None
    return match.group(0) if match else None


def save_global_lockfile(home):
    """The state of the global lockfile before a global cell: its bytes (None
    when absent) and whether the copy `.upskill-lock.json.v1` exists."""
    lock = Path(home) / '.upskill-lock.json'
    return (lock.read_bytes() if lock.exists() else None), (Path(home) / '.upskill-lock.json.v1').exists()


def restore_global_lockfile(home, saved):
    """Return the global lockfile to the state that `save_global_lockfile`
    recorded. `add` adopts a `schema: 1` lockfile to `schema: 2` and writes the
    copy `.v1`, and an older upskill cannot read `schema: 2`."""
    content, had_v1 = saved
    lock = Path(home) / '.upskill-lock.json'
    if content is None:
        lock.unlink(missing_ok=True)
    else:
        lock.write_bytes(content)
    if not had_v1:
        (Path(home) / '.upskill-lock.json.v1').unlink(missing_ok=True)


def smoke_scope(client, scope, version, base, source, upskill, markers, name, args, runner, env):
    """The cells of one client in one scope: install, ask one question per
    kind, and for the global scope remove what was installed and restore the
    global lockfile to its state from before the cell. The lockfile is not
    restored when `upskill remove` fails, and when the lockfile could not be
    read before the cell; each case adds a warning to every cell. When both
    occur, only the removal warning is set."""
    cells = []

    def record(kind, result, reason=None):
        cell = {'client': client, 'kind': kind, 'scope': scope, 'result': result, 'client_version': version or 'unknown'}
        if reason:
            cell['reason'] = reason
        cells.append(cell)

    if scope == 'global' and not args.allow_real_home:
        for kind in args.kinds:
            record(kind, 'unavailable', 'a global cell writes to the real home directory; pass --allow-real-home')
        return cells
    if version is None:
        for kind in args.kinds:
            record(kind, 'unavailable', client + ' --version failed')
        return cells
    cwd = base / (client + '-' + scope)
    cwd.mkdir()
    if scope == 'project':
        subprocess.run(['git', 'init', '-q', str(cwd)], env=env, check=True)
    # `add` is not atomic: a failed or timed-out `add` can leave the lockfile
    # entry and some files. A global cell therefore always runs `remove`.
    home = Path(env.get('HOME') or Path.home())
    saved = None
    save_error = None
    if scope == 'global':
        try:
            saved = save_global_lockfile(home)
        except OSError as error:
            save_error = 'the global lockfile could not be read: ' + str(error)
    try:
        added = runner([upskill, 'add', str(source), '--' + client, '--' + scope], cwd, args.timeout, env)
        if added['state'] != 'success':
            for kind in args.kinds:
                record(kind, 'install-failed', 'upskill add ended with state ' + added['state'])
            return cells
        for kind in args.kinds:
            record(kind, *smoke_cell_result(client, kind, scope, cwd, markers, name, args, runner, env))
    finally:
        if scope == 'global':
            try:
                removed = runner([upskill, 'remove', '--global', '--yes', name], cwd, args.timeout, env)
                if removed['state'] == 'success' or 'not in lockfile: ' + name in removed.get('stderr', ''):
                    # The name is not installed, so nothing is left to remove.
                    failure = None
                else:
                    failure = 'upskill remove ended with state ' + removed['state']
            except subprocess.TimeoutExpired:
                failure = 'upskill remove timed out'
            warning = None
            if failure:
                warning = ('global items ' + name + ' were not removed: ' + failure
                           + '; the global lockfile was left as the run changed it')
            elif save_error:
                warning = 'the global lockfile was not restored: ' + save_error
            else:
                try:
                    restore_global_lockfile(home, saved)
                except Exception as error:
                    warning = 'the global lockfile was not restored: ' + str(error)
            if warning:
                for cell in cells:
                    cell['warning'] = warning
    return cells


def run_smoke(args, runner=invoke):
    """For each selected client and scope: install with
    `<upskill> add <source> --<client> --project|--global`, ask one question
    per kind in a new client session, and return a list of
    {'client', 'kind', 'scope', 'result', 'client_version'} dictionaries; a
    cell can also hold 'reason' (the note text) and 'warning'. A global cell
    then removes the installed items with `upskill remove --global --yes` and
    restores the global lockfile and its `.v1` copy to their state from before
    the cell. It does not restore when the removal fails or when the lockfile
    could not be read before the cell; it then sets 'warning' on each cell, and
    when both occur only the removal warning. A
    project cell runs in a temporary repository that is deleted at the end.
    `result` is 'pass', 'fail', 'timeout', 'unavailable' or 'install-failed';
    'not run' without `--run`."""
    cells = [(client, kind, scope) for client in args.clients for scope in args.scopes for kind in args.kinds]
    if not args.run:
        return [{'client': c, 'kind': k, 'scope': s, 'result': 'not run', 'client_version': '-'} for c, k, s in cells]
    markers = {kind: 'SMOKE_' + kind.upper() + '_' + secrets.token_hex(12) for kind in SMOKE_KINDS}
    name = 'upskill-smoke-' + secrets.token_hex(6)
    env = isolated_git_environment()
    env.pop('CLAUDECODE', None)
    # The runner starts every command in a temporary directory, so a relative
    # path to the binary would not be found.
    upskill = str(Path(args.upskill).resolve())
    results = []
    with tempfile.TemporaryDirectory(prefix='upskill-smoke-') as temporary:
        base = Path(temporary)
        source = smoke_source(base / 'source', name, markers)
        for client in args.clients:
            runnable = [s for s in args.scopes if s == 'project' or args.allow_real_home]
            version = smoke_client_version(client, base, runner, env) if runnable else None
            for scope in args.scopes:
                results.extend(smoke_scope(client, scope, version, base, source, upskill, markers, name, args, runner, env))
    return results


def smoke_main(args, runner=invoke):
    """Print one line for each cell, then the level for each client and kind.
    The exit code is 0 whenever the smoke test ran, whatever the results."""
    results = run_smoke(args, runner)
    for cell in results:
        print(cell['client'], cell['kind'], cell['scope'], cell['result'].replace(' ', '-') if cell['result'] == 'not run' else cell['result'], cell['client_version'])
    for cell in results:
        if cell.get('reason'):
            print('note:', cell['client'], cell['kind'], cell['scope'], cell['reason'])
    for warning in dict.fromkeys(cell['warning'] for cell in results if cell.get('warning')):
        print('warning:', warning)
    if args.run:
        for (client, kind), level in smoke_levels(results).items():
            print(client, kind, level)
    return 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true', help='Explicitly opt in to native model requests')
    parser.add_argument('--smoke', action='store_true', help='Smoke test of installed content against the real clients')
    parser.add_argument('--upskill', type=Path, help='The upskill binary that the smoke test installs with')
    parser.add_argument('--kinds', nargs='+', choices=SMOKE_KINDS, help='With --smoke: kinds to test; default all')
    parser.add_argument('--scopes', nargs='+', choices=SMOKE_SCOPES, help='With --smoke: scopes to test; default both')
    parser.add_argument('--allow-real-home', action='store_true', help='Let global cells install into and remove from the real home directory')
    parser.add_argument('--clients', nargs='+', choices=SMOKE_CLIENTS, default=None)
    parser.add_argument('--timeout', type=int, default=90)
    args = parser.parse_args()
    if not args.smoke:
        given = [flag for flag, value in (('--kinds', args.kinds), ('--scopes', args.scopes),
                                          ('--allow-real-home', args.allow_real_home), ('--upskill', args.upskill)) if value]
        if given:
            parser.error(', '.join(given) + ' only apply with --smoke')
        parser.error('--smoke is required')
    args.kinds = args.kinds or list(SMOKE_KINDS)
    args.scopes = args.scopes or list(SMOKE_SCOPES)
    if not args.upskill:
        parser.error('--upskill is required with --smoke')
    if args.run and not args.upskill.is_file():
        parser.error('--upskill does not name a file: ' + str(args.upskill))
    args.clients = args.clients or list(SMOKE_CLIENTS)
    raise SystemExit(smoke_main(args))
