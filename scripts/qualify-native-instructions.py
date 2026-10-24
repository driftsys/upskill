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


def copilot_plugin_component_fixture(root, rule_suffix='.md'):
    """Prepare Q3 packages and separate scope projects without team guides."""
    if rule_suffix not in ('.md', '.instructions.md'):
        raise ValueError('Only declared native rule filename variants are allowed')
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    root = root.resolve()
    parts = root.parts
    if (root.is_relative_to(Path(__file__).resolve().parents[1])
            or root == Path.home().resolve()
            or any(p in ('.claude', '.github', '.codex', '.opencode') for p in parts)
            or any(parts[i:i + 2] in (('.agents', 'skills'), ('.agents', 'plugins'))
                   for i in range(len(parts) - 1))):
        raise ValueError('A new neutral fixture root is required')
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
        return ['copilot', '--no-auto-update', '--disable-builtin-mcps', '--available-tools=', '--no-ask-user', '--silent', '--log-level', 'none', '-p', prompt]
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
    parser.add_argument('--evidence', type=Path, required=True)
    args = parser.parse_args()
    if not args.run:
        parser.error('--run is required; native probes may use paid model requests')
    run(args)
