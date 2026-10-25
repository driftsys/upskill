#!/usr/bin/env python3
"""Opt-in native instruction discovery probes; no credentials are read or copied."""
import argparse
import hashlib
import importlib.util
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


def opencode_explicit_path_fixtures(root, pin='2.1.4'):
    """Prepare rule lifecycle projects without native discovery or guide writes."""
    spec = importlib.util.spec_from_file_location('plugin_fixture_helpers', Path(__file__).with_name('qualify-plugin-rules.py'))
    helpers = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helpers)
    root = helpers.neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    projects = {label: root / label for label in ('positive', 'project', 'outside')}
    positive = 'NATIVE_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    initial = ['RULE_' + secrets.token_hex(16) for _ in range(2)]
    updated = ['UPDATED_' + secrets.token_hex(16) for _ in range(2)]
    files = [root / 'delivery/rules' / (name + '.md') for name in ('a', 'b')]
    for path, marker in zip(files, initial):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('For the explicit-path qualification question include exactly one occurrence of ' + marker + ' in the rules array. Return only the JSON object with a rules array. Do not use tools.\n')
    env = helpers.fixture_environment(root)
    for label, project in projects.items():
        project.mkdir()
        config = {'permissions': [{'action': '*', 'resource': '*', 'effect': 'deny'}]}
        if label == 'positive':
            native = project / 'native-positive.md'
            native.write_text('For the native qualification question return exactly ' + positive + ' and nothing else. Do not use tools.\n')
            config['instructions'] = [str(native)]
        (project / 'opencode.json').write_text(json.dumps(config, indent=2) + '\n')
        manual = project / 'manual.txt'
        manual.write_text('MANUAL_' + secrets.token_hex(16) + '\n')
        manual.chmod(0o640)
        subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
        subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize explicit-path fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'pin': pin, 'project': projects['project'], 'positive_project': projects['positive'],
            'outside_project': projects['outside'], 'positive_marker': positive, 'fallback': fallback,
            'rule_files': files, 'initial_markers': initial, 'updated_markers': updated}


def classify_opencode_explicit_paths(controls, fixture):
    """Keep observed behavior separate from native file and copy identity."""
    def succeeded(row, project):
        return (isinstance(row, dict) and row.get('pin') == fixture['pin']
                and row.get('state') == 'success' and row.get('completed') is True
                and row.get('tools_observed') is False and row.get('cwd') == str(project)
                and isinstance(row.get('session'), str) and bool(row['session'].strip()))

    positive = controls.get('positive', {})
    if not succeeded(positive, fixture['positive_project']) or positive.get('response') != fixture['positive_marker']:
        return 'positive_control_failed'
    stages = ('unregistered', 'installed', 'resume', 'updated', 'removed', 'reinstalled', 'outside')
    for stage in stages:
        row = controls.get(stage, {})
        project = fixture['outside_project'] if stage == 'outside' else fixture['project']
        if not succeeded(row, project):
            return 'control_failed'
    if controls['resume']['session'] != controls['installed']['session']:
        return 'session_identity_failed'
    fresh = [positive['session'], *(controls[stage]['session'] for stage in stages if stage != 'resume')]
    if len(set(fresh)) != len(fresh):
        return 'session_identity_failed'
    for stage in ('unregistered', 'removed', 'outside'):
        if controls[stage]['response'] != fixture['fallback']:
            return 'absence_control_failed'
    for stage in ('unregistered', 'removed', 'outside'):
        row = controls[stage]
        if row.get('native_instruction_paths') is None or row.get('native_body_counts') is None:
            return 'native_identity_unobserved'
        if row['native_instruction_paths'] != [] or row['native_body_counts'] != {}:
            return 'native_identity_failed'
    for stage in ('installed', 'resume', 'updated', 'reinstalled'):
        row = controls[stage]
        expected = fixture['initial_markers'] if stage in ('installed', 'resume') else fixture['updated_markers']
        response = row['response']
        rules = response.get('rules') if isinstance(response, dict) and set(response) == {'rules'} else None
        if (not isinstance(rules, list) or any(not isinstance(value, str) for value in rules)
                or sorted(rules) != sorted(expected)):
            return 'rule_control_failed'
    for stage in ('installed', 'resume', 'updated', 'reinstalled'):
        row = controls[stage]
        if row.get('native_instruction_paths') is None or row.get('native_body_counts') is None:
            return 'native_identity_unobserved'
        paths, counts = row['native_instruction_paths'], row['native_body_counts']
        if (not isinstance(paths, list) or any(not isinstance(path, str) for path in paths)
                or sorted(paths) != sorted(str(path) for path in fixture['rule_files'])
                or not isinstance(counts, dict) or set(counts) != {'a', 'b'}
                or any(type(counts[key]) is not int or counts[key] != 1 for key in ('a', 'b'))):
            return 'native_identity_failed'
    return 'observed'


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


def copilot_plugin_component_fixture(root, rule_suffix='.md'):
    """Prepare Q3 packages and separate scope projects without team guides."""
    if rule_suffix not in ('.md', '.instructions.md'):
        raise ValueError('Only declared native rule filename variants are allowed')
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    requested = root.expanduser().absolute()
    root = requested.resolve()
    native_homes = ('.claude', '.github', '.codex', '.opencode', '.copilot',
                    '.config/opencode', '.agents/skills', '.agents/plugins')
    forbidden = root.is_relative_to(Path(__file__).resolve().parents[1]) or root == Path.home().resolve()
    for candidate in (requested, root):
        parts = tuple(part.casefold() for part in candidate.parts)
        forbidden |= any(p in ('.claude', '.github', '.codex', '.opencode', '.copilot') for p in parts)
        forbidden |= any(parts[i:i + 2] in (('.agents', 'skills'), ('.agents', 'plugins'), ('.config', 'opencode'))
                         for i in range(len(parts) - 1))
    configured = [Path(value).expanduser().resolve() for key in
                  ('CLAUDE_CONFIG_DIR', 'COPILOT_HOME', 'CODEX_HOME', 'OPENCODE_CONFIG_DIR', 'OPENCODE_CONFIG')
                  if (value := os.environ.get(key))]
    if os.environ.get('XDG_CONFIG_HOME'):
        configured.append(Path(os.environ['XDG_CONFIG_HOME']).expanduser().resolve() / 'opencode')
    configured.extend((Path.home() / name).resolve() for name in native_homes)
    forbidden |= any(root.is_relative_to(location) for location in configured)
    guidance = ('CLAUDE.md', 'AGENTS.md', 'AGENTS.override.md', *native_homes)
    forbidden |= any((parent / name).exists() or (parent / name).is_symlink()
                     for parent in root.parents for name in guidance)
    if forbidden:
        raise ValueError('A new neutral fixture root outside native roots and ancestor guidance is required')
    root.mkdir(parents=True, mode=0o700)
    name = 'upskill-q3-' + secrets.token_hex(8)
    package = root / 'delivery/packages' / name
    package.mkdir(parents=True)
    plain = 'PLAIN_' + secrets.token_hex(16)
    scoped = 'SCOPED_' + secrets.token_hex(16)
    agent_marker = 'AGENT_' + secrets.token_hex(16)
    agent_name = name + '-child'
    catalog_positive_name = name + '-catalog-positive'
    (package / 'plugin.json').write_text(json.dumps({
        '$schema': 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json',
        'name': name, 'version': '0.0.0', 'description': 'Disposable component qualification.'}, indent=2) + '\n')
    rules = package / 'com.github.copilot/rules'
    rules.mkdir(parents=True)
    (rules / ('plain' + rule_suffix)).write_text('---\nname: ' + name + '-plain\n'
                                  'description: Global component qualification rule.\napplyTo: "**"\n---\n'
                                  'For the component qualification question include '
                                  + plain + ' in the rules array.\n')
    (rules / ('scoped' + rule_suffix)).write_text('---\nname: ' + name + '-scoped\n'
                                   'description: Scoped component qualification rule.\napplyTo: "matching/**"\n---\n'
                                   'For the component qualification question include '
                                   + scoped + ' in the rules array.\n')
    agents = package / 'com.github.copilot/agents'
    agents.mkdir(parents=True)
    (agents / 'probe.agent.md').write_text('---\nname: ' + agent_name + '\n'
        'description: Explicit component qualification child.\ntools: [read]\n---\n'
        'Read agent-resource.txt in the current project using the native read tool. '
        'Return its exact contents without other text. Do not edit any file.\n')
    market = root / 'delivery/.claude-plugin/marketplace.json'
    market.parent.mkdir(parents=True)
    market.write_text(json.dumps({'name': name + '-market', 'owner': {'name': 'Qualification Fixture'},
        'plugins': [{'name': name, 'source': './packages/' + name}]}, indent=2) + '\n')
    projects = {}
    native_markers = {}
    for scope in ('project', 'user'):
        project = root / scope
        instructions = project / '.github/copilot-instructions.md'
        instructions.parent.mkdir(parents=True)
        marker = 'NATIVE_' + secrets.token_hex(16)
        native_markers[scope] = marker
        instructions.write_text('For the component qualification question return only a JSON object '
            'with "native": "' + marker + '" and "rules": an array of markers from '
            'automatically loaded component qualification rules, sorted lexicographically. '
            'Without those rules the array is empty. Only use tools explicitly requested.\n')
        catalog_positive = project / '.github/agents/catalog-positive.agent.md'
        catalog_positive.parent.mkdir(parents=True)
        catalog_positive.write_text('---\nname: ' + catalog_positive_name + '\n'
            'description: Native catalog positive for a disposable fixture.\ntools: [read]\n---\n'
            'Read agent-resource.txt and return its exact contents. Do not edit files.\n')
        for directory in ('matching', 'nonmatching'):
            resource = project / directory / 'probe.txt'
            resource.parent.mkdir(parents=True)
            resource.write_text('Disposable scope resource.\n')
        (project / 'agent-resource.txt').write_text(agent_marker + '\n')
        env = isolated_git_environment()
        subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
        subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture',
                        '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null',
                        '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m',
                        'test: initialize component fixture ' + secrets.token_hex(16)], env=env, check=True)
        projects[scope] = project
    return {'root': root, 'package': package, 'marketplace': root / 'delivery',
            'plugin_name': name, 'market_name': name + '-market', 'projects': projects,
            'native_markers': native_markers, 'plain_marker': plain, 'scoped_marker': scoped,
            'agent_name': agent_name, 'agent_marker': agent_marker,
            'catalog_positive_name': catalog_positive_name, 'pin': '1.0.103', 'rule_suffix': rule_suffix}


def copilot_plugin_agent_selection(config_options, plugin_name, agent_name):
    """Keep native namespaced selection values distinct from display names."""
    matches = []
    for option in config_options:
        if option.get('id') != 'agent':
            continue
        for choice in option.get('options', []):
            value = choice.get('value')
            if (choice.get('name') == agent_name and isinstance(value, str)
                    and value.startswith(plugin_name + ':') and value != plugin_name + ':'):
                matches.append(value)
    return matches[0] if len(matches) == 1 else None


def copilot_delegation_policy(tool_name, arguments, expected_agent, expected_model, resource):
    """Bound a private Q3 retry; this is not a production permission adapter."""
    if (not isinstance(tool_name, str) or not isinstance(arguments, dict)
            or not isinstance(expected_agent, str) or not expected_agent
            or not isinstance(expected_model, str) or not expected_model):
        return 'deny'
    if tool_name.lower() == 'task':
        if (arguments.get('agent_type') != expected_agent
                or arguments.get('model') != expected_model
                or any(arguments.get(k) is not None for k in ('resume', 'agent_id', 'session_id'))
                or arguments.get('mode') not in (None, 'sync', 'foreground')):
            return 'deny'
        return 'allow'
    if tool_name.lower() in ('view', 'read', 'read_file'):
        paths = [arguments[k] for k in ('path', 'file_path') if k in arguments]
        if len(paths) != 1 or not isinstance(paths[0], str) or not paths[0]:
            return 'deny'
        path = Path(paths[0])
        if not path.is_absolute():
            path = resource.parent / path
        return 'allow' if path.resolve() == resource.resolve() else 'deny'
    return 'deny'


def classify_copilot_plugin_components(observations, fixture, scope):
    """Classify one Q3 scope, preserving failed positives and missing controls."""
    def succeeded(row):
        return (row.get('state') == 'success' and row.get('completed') is True
                and row.get('pin') == fixture['pin'])

    native = fixture['native_markers'][scope]
    positive = observations.get('native_positive', {})
    if not succeeded(positive) or positive.get('response') != {'native': native, 'rules': []}:
        return 'positive_control_failed'
    for stage in ('installed', 'matching', 'nonmatching', 'disabled', 'uninstalled', 'reinstalled'):
        row = observations.get(stage, {})
        if not succeeded(row):
            return 'control_failed'
        rules = [] if stage in ('disabled', 'uninstalled') else [fixture['plain_marker']]
        if stage == 'matching':
            rules.append(fixture['scoped_marker'])
        if row.get('response') != {'native': native, 'rules': sorted(rules)}:
            return 'component_response_mismatch'
        if stage in ('matching', 'nonmatching') and row.get('resource_read') is not True:
            return 'resource_read_missing'
    agent = observations.get('agent', {})
    if (not succeeded(agent) or agent.get('response') != fixture['agent_marker']
            or any(agent.get(k) is not True for k in ('catalog_present', 'delegated', 'child_resource_read'))):
        return 'agent_control_failed'
    return 'observed'



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
