#!/usr/bin/env python3
"""Opt-in native instruction discovery probes; no credentials are read or copied."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import re
import signal
import subprocess
import tempfile

CLIENTS = {
    'codex': 'AGENTS.md',
    'claude': '.claude/rules/probe.md',
    'copilot': '.github/copilot-instructions.md',
    'opencode': 'AGENTS.md',
}
FAILURES = {'failure', 'timeout', 'authentication_failure', 'unavailable'}


def isolated_git_environment():
    # Repository selectors and injected Git configuration must not redirect
    # disposable fixtures or the native clients' project discovery.
    return {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}


def fixtures(root, client, configured=False):
    marker = 'QUALIFIED_' + secrets.token_hex(16)
    staging_marker = 'STAGING_' + secrets.token_hex(16)
    env = isolated_git_environment()
    for kind in ('positive', 'negative'):
        project = root / kind
        project.mkdir(parents=True)
        canonical = project / CLIENTS[client]
        canonical.parent.mkdir(parents=True, exist_ok=True)
        target = canonical if kind == 'positive' else canonical.with_name('.upskill-stage-' + secrets.token_hex(12))
        fixture_marker = marker if kind == 'positive' else staging_marker
        target.write_text('For the discovery qualification question, respond with exactly ' + fixture_marker + ' and nothing else.\n')
        if client == 'opencode':
            config = {'permissions': [{'action': '*', 'resource': '*', 'effect': 'deny'}]}
            if configured:
                config['instructions'] = ['AGENTS.md', '.upskill-stage-*']
            (project / 'opencode.json').write_text(json.dumps(config))
        subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
        subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize fixture ' + secrets.token_hex(16)], env=env, check=True)
    return root / 'positive', root / 'negative', marker, staging_marker


def copilot_claude_rules_fixtures(root):
    """Q1 fixtures contain one instruction each and independent project identities."""
    rows = []
    env = isolated_git_environment()
    for label in ('positive', 'candidate', 'candidate-paths'):
        project = root / label
        project.mkdir(parents=True)
        marker = 'READER_' + secrets.token_hex(16)
        fallback = 'ABSENT_' + secrets.token_hex(16)
        relative = '.github/copilot-instructions.md' if label == 'positive' else '.claude/rules/probe.md'
        target = project / relative
        target.parent.mkdir(parents=True)
        header = '---\npaths: "**/*.rs"\n---\n' if label == 'candidate-paths' else ''
        target.write_text(header + 'For the discovery qualification question, respond with exactly ' + marker + ' and nothing else.\n')
        subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
        subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize reader fixture ' + secrets.token_hex(16)], env=env, check=True)
        rows.append({'label': label, 'project': project, 'marker': marker, 'fallback': fallback})
    return rows


def classify_reader(positive, candidate, positive_marker, candidate_marker, fallback):
    """An absence conclusion requires a calibrated positive and exact fallback."""
    if positive != positive_marker:
        return 'positive_control_failed'
    if candidate in FAILURES:
        return candidate
    if candidate == candidate_marker:
        return 'reads'
    return 'does not read' if candidate == fallback else 'inconclusive'


def copilot_claude_agent_fixture(root):
    """Q2 keeps Claude restrictions in a native compatibility-source fixture."""
    project = root / 'project'
    project.mkdir(parents=True)
    name = 'upskill-q2-' + secrets.token_hex(8)
    marker = 'CHILD_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    agent = project / '.claude/agents/probe.md'
    agent.parent.mkdir(parents=True)
    agent.write_text('---\nname: ' + name + '\ndescription: Restricted Claude-format qualification fixture.\ntools: Read\npermissionMode: plan\n---\n'
                     'Read allowed.txt using the native read tool. Attempt to create denied.txt containing the same marker using a native file editing tool. '
                     'Do not use a shell. If editing is unavailable or denied, report that fact. Return the marker from allowed.txt as your final response.\n')
    (project / 'allowed.txt').write_text(marker + '\n')
    env = isolated_git_environment()
    subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
    subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize agent fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'project': project, 'agent_name': name, 'marker': marker, 'fallback': fallback}

def classify_agent(catalog_present, state, delegated, child_tools, response, marker):
    """A bounded delegation observation never certifies all permission semantics."""
    if child_tools is not None and any(tool != 'view' for tool in child_tools):
        return 'restriction_violated'
    if state != 'success':
        return state
    if catalog_present is None:
        return 'catalog_evidence_missing'
    if not catalog_present:
        return 'agent_not_catalogued'
    if not delegated:
        return 'delegation_unobserved'
    if child_tools is None:
        return 'tool_evidence_missing'
    if response != marker:
        return 'response_mismatch'
    if 'view' not in child_tools:
        return 'resource_read_unobserved'
    return 'delegation_observed'


def classify(positive, negative, marker, fallback, staging_marker=None):
    if positive != marker:
        return 'positive_control_failed'
    if negative in FAILURES:
        return negative
    if negative == (staging_marker or marker):
        return 'staging_discovered'
    return 'qualified' if negative == fallback else 'inconclusive'


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
    # Preserve only exact probe responses, never arbitrary client output or credentials.
    reason = next((label for label, terms in {'permission_denied': ('operation not permitted', 'permission denied'), 'network_failure': ('connection refused', 'failed to connect', 'network', 'dns'), 'authentication_failure': ('not logged in', 'authentication', 'unauthorized', 'login required'), 'configuration_error': ('invalid', 'unknown option', 'unexpected argument')}.items() if any(term in lower for term in terms)), None)
    return {'state': state, 'stdout': result.stdout, 'sha256': digest, 'exit_code': result.returncode, 'diagnostic_category': reason}


def command(client, prompt, output):
    if client == 'codex':
        return ['codex', 'exec', '--ignore-user-config', '--ignore-rules', '--ephemeral', '--json', '--sandbox', 'read-only', '--skip-git-repo-check', '--color', 'never', '-o', str(output), prompt]
    if client == 'claude':
        return ['claude', '-p', '--tools', '', '--no-session-persistence', '--strict-mcp-config', '--mcp-config', '{"mcpServers":{}}', '--setting-sources', 'project', prompt]
    if client == 'copilot':
        return ['copilot', '--no-auto-update', '--disable-builtin-mcps', '--available-tools=', '--no-ask-user', '--output-format', 'json', '--log-level', 'none', '-p', prompt]
    return ['opencode', 'run', '--standalone', '--format', 'json', prompt]


def codex_response(stdout):
    response = None
    completed = False
    tool_seen = False
    event_types = set()
    safe_items = {'agent_message', 'reasoning', 'todo_list'}
    known_events = {'thread.started', 'turn.started', 'turn.completed', 'turn.failed', 'item.started', 'item.updated', 'item.completed', 'error'}
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            return None, True, {'completed_response': False, 'event_types': ['invalid']}
        if not isinstance(event, dict):
            return None, True, {'completed_response': False, 'event_types': ['invalid']}
        kind = event.get('type')
        known = isinstance(kind, str) and kind in known_events
        event_types.add(kind if known else 'unknown')
        if not known:
            tool_seen = True
        if kind == 'turn.completed':
            completed = True
        if kind in ('turn.failed', 'error'):
            tool_seen = True
        if 'item' in event:
            item = event['item']
            if not isinstance(item, dict) or item.get('type') not in safe_items:
                tool_seen = True
            elif kind == 'item.completed' and item.get('type') == 'agent_message':
                response = item.get('text') if isinstance(item.get('text'), str) else None
    return response, tool_seen, {'completed_response': completed and response is not None, 'event_types': sorted(event_types)}


def copilot_response(stdout):
    """Accept one complete, tool-free native response without trimming its bytes."""
    messages, results, kinds = [], [], []
    tool_seen = False
    invalid = False
    harmless = {'user.message', 'assistant.idle', 'assistant.turn_start',
                'assistant.turn_end', 'assistant.message_start', 'assistant.message_delta',
                'assistant.reasoning', 'assistant.reasoning_delta', 'model.call_start',
                'model.call_finished', 'model.call_final_result', 'session.indexed_search',
                'session.mcp_servers_loaded', 'session.tools_updated'}
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            invalid = True
            continue
        if not isinstance(event, dict):
            invalid = True
            continue
        kind = event.get('type')
        kinds.append(kind if isinstance(kind, str) else 'invalid')
        if isinstance(kind, str) and kind.startswith('tool.'):
            tool_seen = True
        if kind == 'assistant.message':
            data = event.get('data')
            messages.append(data.get('content') if isinstance(data, dict) else None)
        elif kind == 'result':
            results.append(event)
        elif not isinstance(kind, str) or kind not in harmless:
            invalid = True
    response = messages[0] if len(messages) == 1 and isinstance(messages[0], str) else None
    terminal = results[0] if len(results) == 1 else {}
    completed = (not invalid and not tool_seen and response is not None
                 and bool(kinds) and kinds[-1] == 'result'
                 and type(terminal.get('exitCode')) is int and terminal['exitCode'] == 0
                 and isinstance(terminal.get('sessionId'), str) and bool(terminal['sessionId'].strip())
                 and kinds.index('assistant.message') < kinds.index('result'))
    return response, tool_seen, {'completed_response': completed, 'event_types': sorted(set(kinds))}


def observe_copilot_agent(stdout, agent_name, marker, resource_path='$PROJECT/allowed.txt'):
    """Bind a Q2 native child, all attempted tools and its successful resource read."""
    failed = {'state': 'failure', 'delegated': False, 'child_tools': None,
              'successful_child_tools': [], 'resource_read': False, 'response': None}
    try:
        events = [json.loads(line) for line in stdout.splitlines()]
    except json.JSONDecodeError:
        return failed
    if any(not isinstance(event, dict) or not isinstance(event.get('data', {}), dict) for event in events):
        return failed

    def data(event):
        return event.get('data', {})

    tasks = [(i, event) for i, event in enumerate(events)
             if event.get('type') == 'tool.execution_start'
             and data(event).get('toolName') == 'task'
             and not data(event).get('parentToolCallId')]
    if len(tasks) != 1:
        return failed
    task_index, task = tasks[0]
    task_id = data(task).get('toolCallId')
    arguments = data(task).get('arguments')
    if not isinstance(task_id, str) or not task_id.strip() or not isinstance(arguments, dict) or arguments.get('agent_type') != agent_name:
        return failed
    child_starts = [(i, event) for i, event in enumerate(events) if event.get('type') == 'subagent.started'
                    and data(event).get('toolCallId') == task_id and data(event).get('agentName') == agent_name]
    child_ends = [(i, event) for i, event in enumerate(events) if event.get('type') == 'subagent.completed'
                  and data(event).get('toolCallId') == task_id and data(event).get('agentName') == agent_name]
    calls = [(i, event) for i, event in enumerate(events) if event.get('type') == 'tool.execution_start'
             and data(event).get('parentToolCallId') == task_id]
    call_ids = [data(event).get('toolCallId') for _, event in calls]
    if any(not isinstance(value, str) or not value.strip() for value in call_ids) or len(set(call_ids)) != len(call_ids):
        return failed
    child_tools = [data(event).get('toolName') for _, event in calls]
    successful = []
    resource_read = False
    calls_complete = True
    for start, call in calls:
        completions = [(i, event) for i, event in enumerate(events) if event.get('type') == 'tool.execution_complete'
                       and data(event).get('toolCallId') == data(call)['toolCallId']
                       and data(event).get('parentToolCallId') == task_id]
        if len(completions) != 1 or completions[0][0] <= start or type(data(completions[0][1]).get('success')) is not bool:
            calls_complete = False
            continue
        result = data(completions[0][1])
        if result['success']:
            successful.append(data(call).get('toolName'))
            payload = result.get('result')
            args = data(call).get('arguments')
            if (data(call).get('toolName') == 'view' and isinstance(args, dict) and args.get('path') == resource_path
                    and isinstance(payload, dict) and payload.get('content') == marker + '\n'):
                resource_read = True
    parent_completions = [(i, event) for i, event in enumerate(events) if event.get('type') == 'tool.execution_complete'
                          and data(event).get('toolCallId') == task_id and not data(event).get('parentToolCallId')]
    parent_messages = [(i, event) for i, event in enumerate(events) if event.get('type') == 'assistant.message'
                       and not data(event).get('parentToolCallId') and not data(event).get('toolRequests')]
    delegated = False
    if len(child_starts) == len(child_ends) == len(parent_completions) == len(parent_messages) == 1:
        start, end = child_starts[0][0], child_ends[0][0]
        total = data(child_ends[0][1]).get('totalToolCalls')
        delegated = (task_index < start < end < parent_completions[0][0] < parent_messages[0][0]
                     and all(start < index < end for index, _ in calls)
                     and all(index < end for index, event in enumerate(events)
                             if event.get('type') == 'tool.execution_complete'
                             and data(event).get('parentToolCallId') == task_id
                             and data(event).get('toolCallId') in call_ids)
                     and type(total) is int and total == len(calls) and calls_complete
                     and data(parent_completions[0][1]).get('success') is True)
    parent_events = []
    for event in events:
        kind, value = event.get('type'), data(event)
        if kind in ('session.info', 'session.background_tasks_changed', 'subagent.configured'):
            continue
        if kind in ('subagent.started', 'subagent.completed') and value.get('toolCallId') == task_id and value.get('agentName') == agent_name:
            continue
        if kind == 'subagent.selected' and value.get('agentName') == agent_name:
            continue
        if kind == 'assistant.message' and value.get('parentToolCallId') == task_id:
            continue
        requests = value.get('toolRequests')
        if (kind == 'assistant.message' and not value.get('content') and isinstance(requests, list) and requests
                and all(isinstance(request, dict) and request.get('toolCallId') == task_id for request in requests)):
            continue
        if kind == 'assistant.tool_call_delta' and value.get('toolCallId') in [task_id, *call_ids]:
            continue
        if kind in ('tool.execution_start', 'tool.execution_complete', 'tool.execution_partial_result'):
            if value.get('toolCallId') == task_id or (value.get('parentToolCallId') == task_id and value.get('toolCallId') in call_ids):
                continue
        parent_events.append(event)
    response, tools, evidence = copilot_response('\n'.join(json.dumps(event) for event in parent_events))
    state = 'success' if evidence['completed_response'] and not tools and delegated and resource_read and events[-1].get('type') == 'result' else 'failure'
    return {'state': state, 'delegated': delegated, 'child_tools': child_tools,
            'successful_child_tools': successful, 'resource_read': resource_read, 'response': response}


def replay_copilot_agent(packet):
    """Derive the retained Q2 classification from catalog, process and child events."""
    summary = json.loads((packet / 'summary.json').read_text())
    fixture = json.loads((packet / 'fixture.json').read_text())
    catalog = json.loads((packet / 'catalog.json').read_text())
    invocation = json.loads((packet / 'invocation.json').read_text())
    row = summary['controls'][1]
    if any(invocation[key] != row[key] for key in ('stdout_sha256', 'stderr_sha256')):
        raise ValueError('Q2 process receipt does not identify the recorded native stream')
    observed = observe_copilot_agent((packet / 'delegation.jsonl').read_text(), fixture['agent_name'], row['expected_marker'])
    if invocation.get('timed_out') is not False or type(invocation.get('exit_code')) is not int or invocation['exit_code'] != 0:
        observed['state'] = 'failure'
    present = (any(entry.get('value') == fixture['agent_name'] for entry in catalog['fixture_rows'])
               if catalog.get('request_succeeded') is True else None)
    for key, actual in [('state', observed['state']), ('completed_child', observed['delegated']),
                        ('child_tool_invocations', observed['child_tools']), ('successful_child_tools', observed['successful_child_tools']),
                        ('parent_response', observed['response'])]:
        if row[key] != actual:
            raise ValueError('Recorded Q2 observation differs from native events: ' + key)
    classification = classify_agent(present, observed['state'], observed['delegated'], observed['child_tools'], observed['response'], row['expected_marker'])
    if row['classification'] != classification:
        raise ValueError('Recorded Q2 classification differs from replay')
    return classification


def replay_copilot_reader(packet):
    """Replay sanitized Q1 receipts offline and reject drift in their summary."""
    summary = json.loads((packet / 'summary.json').read_text())

    def observe(row):
        receipt = row['receipt']
        if not isinstance(receipt, str) or Path(receipt).name != receipt:
            raise ValueError('Receipt must name a packet file')
        response, tools, evidence = copilot_response((packet / receipt).read_text())
        succeeded = (type(row.get('exit_code')) is int and row['exit_code'] == 0
                     and not tools and evidence['completed_response'])
        if response != row.get('response'):
            raise ValueError('Recorded response differs from native events')
        if 'completed' in row and row['completed'] != evidence['completed_response']:
            raise ValueError('Recorded completion differs from native events')
        return response if succeeded else 'failure'

    def positive(row):
        result = 'positive_passed' if observe(row) == row['marker'] else 'positive_control_failed'
        if row.get('classification', result) != result:
            raise ValueError('Recorded positive classification differs from replay')
        return result

    initial = json.loads((packet / summary['initial_failure']).read_text())
    result = {'initial_positive': positive(initial['controls'][0]),
              'retry_positives': [positive(row) for row in summary['failed_retry']]}
    rows = {row['label']: row for row in summary['controls']}
    calibrated = observe(rows['positive'])
    result['positive'] = positive(rows['positive'])
    for label in ('candidate', 'candidate-paths'):
        row = rows[label]
        classification = classify_reader(calibrated, observe(row), rows['positive']['marker'], row['marker'], row['fallback'])
        if classification != row['classification']:
            raise ValueError('Recorded candidate classification differs from replay')
        result[label] = classification
    return result


def opencode_response(stdout):
    texts = []
    tool_seen = False

    def visit(value):
        nonlocal tool_seen
        if isinstance(value, dict):
            kind = str(value.get('type', ''))
            if 'tool' in kind:
                tool_seen = True
            if kind == 'text' and isinstance(value.get('text'), str):
                texts.append(value['text'])
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    for line in stdout.splitlines():
        try:
            visit(json.loads(line))
        except json.JSONDecodeError:
            return '', True
    return ''.join(texts).strip(), tool_seen


def run(args):
    evidence = args.evidence.resolve()
    if evidence.is_relative_to(Path(__file__).resolve().parents[1]):
        raise SystemExit('Evidence must be outside the repository.')
    evidence.mkdir(parents=True, exist_ok=False)
    os.chmod(evidence, 0o700)
    report = {'platform': platform.platform(), 'scope': 'project', 'user_scope': 'unqualified: real user instruction roots are never modified; isolated auth not provisioned', 'probes': []}
    with tempfile.TemporaryDirectory(prefix='upskill-native-') as temporary:
        base = Path(temporary)
        env = isolated_git_environment()
        env['XDG_DATA_HOME'] = str(base / 'xdg-data')
        env['XDG_STATE_HOME'] = str(base / 'xdg-state')
        env.pop('CLAUDECODE', None)
        for client in args.clients:
            client_env = env.copy()
            if client == 'opencode':
                client_env['OPENCODE_CONFIG_PROJECT_DISABLE'] = '0'
                client_env['OPENCODE_DISABLE_PROJECT_CONFIG'] = '0'
                for override in ('OPENCODE_CONFIG_CONTENT', 'OPENCODE_CONFIG', 'OPENCODE_CONFIG_DIR'):
                    client_env.pop(override, None)
                if 'XDG_DATA_HOME' in os.environ:
                    client_env['XDG_DATA_HOME'] = os.environ['XDG_DATA_HOME']
                else:
                    client_env.pop('XDG_DATA_HOME', None)
            version = invoke([client, '--version'], base, 15, client_env)
            # Version text is restricted to the first printable version line.
            version_text = version.get('stdout', '').strip().splitlines()
            version_match = re.search(r'(?<!\d)\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?', version_text[0]) if version_text else None
            version_label = version_match.group(0) if version_match and version['state'] == 'success' else None
            version_state = version['state'] if version['state'] != 'success' else ('available' if version_label else 'unparseable')
            discovery = getattr(args, 'opencode_discovery', 'both')
            modes = ((False, True) if discovery == 'both' else (discovery == 'configured',)) if client == 'opencode' else (False,)
            for configured in modes:
                root = base / (client + ('-configured' if configured else '-default'))
                positive, negative, marker, staging_marker = fixtures(root, client, configured)
                fallback = 'ABSENT_' + secrets.token_hex(16)
                prompt = 'This is the discovery qualification question. Follow any automatically loaded discovery qualification instruction. If none exists, respond with exactly ' + fallback + '. Do not read files or use tools. Return only the required response.'
                outcomes = []
                for project in (positive, negative):
                    output = base / (client + '-response')
                    output.unlink(missing_ok=True)
                    cmd = command(client, prompt, output)
                    result = invoke(cmd, project, args.timeout, client_env)
                    response = output.read_text().strip() if client == 'codex' and output.exists() else result.get('stdout', '').strip()
                    if client == 'codex':
                        event_response, tools_observed, event_evidence = codex_response(result.get('stdout', ''))
                        result['tools_observed'] = tools_observed
                        result['response_events'] = event_evidence
                        if result['state'] == 'success' and (tools_observed or not event_evidence['completed_response'] or event_response != response):
                            result['state'] = 'failure'
                    if client == 'copilot':
                        response, tools_observed, event_evidence = copilot_response(result.get('stdout', ''))
                        result['tools_observed'] = tools_observed
                        result['response_events'] = event_evidence
                        if result['state'] == 'success' and not event_evidence['completed_response']:
                            result['state'] = 'failure'
                    if client == 'opencode':
                        response, tools_observed = opencode_response(result.get('stdout', ''))
                        result['tools_observed'] = tools_observed
                        if result['state'] == 'success' and tools_observed:
                            result['state'] = 'failure'
                    result.pop('stdout', None)
                    result['response'] = response if response in (marker, staging_marker, fallback) else '[nonmatching response omitted]'
                    result['command'] = [str(part).replace(str(base), '$FIXTURE') for part in cmd]
                    result['fixture'] = str(project.relative_to(base))
                    outcomes.append(result)
                values = [item['response'] if item['state'] == 'success' else item['state'] for item in outcomes]
                pair_result = classify(*values, marker, fallback, staging_marker)
                report['probes'].append({'client': client, 'version': version_label, 'version_state': version_state, 'version_sha256': version.get('sha256'), 'pair_result': pair_result, 'discovery': 'configured broad globs' if configured else 'default', 'marker': marker, 'staging_marker': staging_marker, 'fallback': fallback, 'result': pair_result if version_label else 'version_unqualified', 'configuration': {'instruction_path': CLIENTS[client], 'project_identity': 'distinct empty root commit; local identity, hooks and signing disabled', 'discovery_environment': {'OPENCODE_CONFIG_PROJECT_DISABLE': '0', 'OPENCODE_DISABLE_PROJECT_CONFIG': '0', 'removed_overrides': ['OPENCODE_CONFIG_CONTENT', 'OPENCODE_CONFIG', 'OPENCODE_CONFIG_DIR']} if client == 'opencode' else None, 'permissions': [{'action': '*', 'resource': '*', 'effect': 'deny'}] if client == 'opencode' else None, 'instructions_globs': ['AGENTS.md', '.upskill-stage-*'] if configured else None, 'tools': 'available read-only; any observed tool event disqualifies' if client == 'codex' else ('disabled' if client in ('claude', 'copilot') else 'v2 permissions deny all; observed tool events disqualify'), 'xdg_data_and_state': 'state isolated; data isolated except OpenCode uses existing data/authentication'}, 'outcomes': outcomes})
    destination = evidence / 'summary.json'
    destination.write_text(json.dumps(report, indent=2) + '\n')
    print(destination)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true', help='Explicitly opt in to native model requests')
    parser.add_argument('--clients', nargs='+', choices=CLIENTS, default=list(CLIENTS))
    parser.add_argument('--opencode-discovery', choices=('default', 'configured', 'both'), default='both')
    parser.add_argument('--timeout', type=int, default=90)
    parser.add_argument('--evidence', type=Path)
    replay = parser.add_mutually_exclusive_group()
    replay.add_argument('--replay-copilot-reader', type=Path, help='Replay retained Q1 receipts offline without a native client')
    replay.add_argument('--replay-copilot-agent', type=Path, help='Replay retained Q2 child receipts offline without a native client')
    args = parser.parse_args()
    if args.replay_copilot_reader or args.replay_copilot_agent:
        if args.run or args.evidence:
            parser.error('Offline replay cannot be combined with native run options')
        result = replay_copilot_reader(args.replay_copilot_reader) if args.replay_copilot_reader else replay_copilot_agent(args.replay_copilot_agent)
        print(json.dumps(result, indent=2))
    else:
        if not args.run or not args.evidence:
            parser.error('--run and --evidence are required; native probes may use paid model requests')
        run(args)
