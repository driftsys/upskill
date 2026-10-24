#!/usr/bin/env python3
"""Prepare disposable native delivery fixtures; offline results never qualify clients."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import stat
import subprocess


REPO = next((p for p in Path(__file__).resolve().parents if (p / '.git').is_dir()),
            Path(__file__).resolve().parents[1])
TEMPLATES = Path(__file__).resolve().parents[1] / 'tests/fixtures/native-plugins'
VERSIONS = {'codex': '0.163.0', 'opencode': '2.0.32'}


def snapshot(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError('Team instructions must be a regular owned fixture file')
    return {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
            'mode': stat.S_IMODE(path.stat().st_mode)}


def classify_pair(positive, control, *, expected_positive, expected_control,
                  positive_counts, control_counts, team_before, version):
    """Classify one paired observation, never the whole qualification campaign."""
    def succeeded(value):
        return (value.get('state') == 'success' and value.get('completed') is True
                and value.get('tools_observed') is False and value.get('version') == version)

    if not succeeded(positive):
        return 'positive_control_failed'
    if not succeeded(control):
        return 'control_failed'
    if positive.get('team') != team_before or control.get('team') != team_before:
        return 'team_changed'
    if positive.get('response') != expected_positive:
        return 'positive_response_mismatch'
    if control.get('response') != expected_control:
        return 'control_response_mismatch'
    for observation, expected in ((positive, positive_counts), (control, control_counts)):
        counts = observation.get('activation_counts')
        if (not isinstance(counts, dict) or counts != expected
                or any(type(n) is not int or n < 1 for n in counts.values())):
            return 'activation_count_mismatch'
    return 'observed'


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n')


def fixture_environment(root):
    """Use only declared disposable roots; do not inherit auth/config overrides."""
    root = root.resolve()
    env = {k: v for k, v in os.environ.items()
           if k in ('PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'TERM', 'SystemRoot')}
    home = root / 'home'
    env.update(HOME=str(home), USERPROFILE=str(home), CODEX_HOME=str(home / '.codex'),
               CLAUDE_CONFIG_DIR=str(home / '.claude'), COPILOT_HOME=str(home / '.copilot'),
               XDG_CONFIG_HOME=str(home / '.config'), XDG_DATA_HOME=str(home / '.local/share'),
               XDG_STATE_HOME=str(home / '.local/state'), XDG_CACHE_HOME=str(home / '.cache'),
               DISABLE_AUTOUPDATER='1')
    return env


def neutral_fixture_root(root):
    """Validate a new campaign root before any fixture writes."""
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    requested = root.expanduser().absolute()
    root = requested.resolve()
    if root.exists() or root == Path.home().resolve():
        raise ValueError('Fixture root must not exist or be the personal home')
    forbidden = False
    for candidate in (requested, root):
        parts = tuple(part.casefold() for part in candidate.parts)
        forbidden |= any(part in ('.claude', '.github', '.codex', '.copilot', '.opencode') for part in parts)
        forbidden |= any(parts[i:i + 2] in (('.agents', 'skills'), ('.agents', 'plugins'), ('.config', 'opencode')) for i in range(len(parts) - 1))
    configured = [Path(value).expanduser().resolve() for key in
                  ('CLAUDE_CONFIG_DIR', 'COPILOT_HOME', 'CODEX_HOME', 'OPENCODE_CONFIG_DIR', 'OPENCODE_CONFIG')
                  if (value := os.environ.get(key))]
    if os.environ.get('XDG_CONFIG_HOME'):
        configured.append(Path(os.environ['XDG_CONFIG_HOME']).expanduser().resolve() / 'opencode')
    native_roots = ('.claude', '.copilot', '.codex', '.opencode', '.github', '.agents/skills', '.agents/plugins', '.config/opencode')
    configured.extend((Path.home() / name).resolve() for name in native_roots)
    resolved_parts = tuple(part.casefold() for part in root.parts)
    forbidden |= any(resolved_parts[:len(location.parts)] == tuple(part.casefold() for part in location.parts)
                     for location in [REPO, *configured])
    guidance = ('CLAUDE.md', 'AGENTS.md', 'AGENTS.override.md', *native_roots)
    forbidden |= any((parent / name).exists() or (parent / name).is_symlink()
                     for parent in root.parents for name in guidance)
    if forbidden:
        raise ValueError('A new neutral fixture root outside client discovery and the repository is required')
    return root


def prepare_claude_managed_root(root):
    """Build a neutral, copy-only Q6 marketplace without team guidance files."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    nonce = secrets.token_hex(8)
    plugin = 'upskill-q6-' + nonce
    market = 'upskill-q6-market-' + nonce
    skill = 'probe-' + nonce
    initial = 'INITIAL_' + secrets.token_hex(16)
    edited = 'EDITED_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    manual = 'MANUAL_' + secrets.token_hex(16)
    managed = root / 'delivery'
    package = managed / 'packages' / plugin
    write_json(managed / '.claude-plugin/marketplace.json', {'name': market, 'owner': {'name': 'Qualification Fixture'}, 'plugins': [{'name': plugin, 'source': './packages/' + plugin}]})
    write_json(package / '.claude-plugin/plugin.json', {'name': plugin, 'version': '0.0.0', 'description': 'Disposable managed-root qualification fixture.'})
    skill_path = package / 'skills' / skill / 'SKILL.md'
    skill_path.parent.mkdir(parents=True)
    skill_path.write_text('---\nname: ' + skill + '\ndescription: Explicit managed-root qualification skill.\n---\nReturn exactly ' + initial + ' and nothing else. Do not use any other tools.\n')
    projects = [root / 'project', root / 'user-project']
    env = fixture_environment(root)
    for project in projects:
        settings = project / '.claude/settings.json'
        write_json(settings, {'env': {'UPSKILL_Q6_MANUAL': manual}})
        settings.chmod(0o640)
        subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
        subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize managed-root fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'managed_root': managed, 'package': package, 'project': projects[0], 'user_project': projects[1], 'plugin_name': plugin, 'market_name': market, 'skill_name': skill, 'skill': skill_path, 'initial_marker': initial, 'edited_marker': edited, 'fallback': fallback, 'manual': manual}


def classify_claude_settings(before, after, before_mode, after_mode):
    """Allow only added empty native registration objects as declared residue."""
    if before == after and before_mode == after_mode:
        return 'unchanged'
    if after is None or (before is not None and before_mode != after_mode):
        return 'preservation_failed'
    try:
        old = json.loads(before) if before is not None else {}
        new = json.loads(after)
    except (ValueError, TypeError):
        return 'preservation_failed'
    if not isinstance(old, dict) or not isinstance(new, dict):
        return 'preservation_failed'
    added = set(new) - set(old)
    permitted = {'enabledPlugins', 'extraKnownMarketplaces'}
    if added - permitted or any(new[key] != {} for key in added):
        return 'preservation_failed'
    remaining = {key: value for key, value in new.items() if key not in added}
    if json.dumps(remaining, sort_keys=True) != json.dumps(old, sort_keys=True):
        return 'preservation_failed'
    return 'empty_object_residue' if added else 'preservation_failed'


def prepare_codex_rule_lifecycle(root):
    """Prepare Q9 without writing team guidance or assuming hook trust."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    project = root / 'project'
    project.mkdir()
    env = fixture_environment(root)
    subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
    subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture',
                    '-c', 'user.email=qualification@example.invalid',
                    '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false',
                    'commit', '--quiet', '--allow-empty', '-m',
                    'test: initialize rule lifecycle fixture ' + secrets.token_hex(16)],
                   env=env, check=True)
    nonce = secrets.token_hex(8)
    market = 'upskill-q9-' + nonce
    packages = []
    bodies = {}
    for rule in ('a', 'b'):
        package = root / 'delivery/packages' / (market + '-' + rule)
        package.mkdir(parents=True)
        body = ('For the rule lifecycle question include exactly one occurrence of '
                + 'RULE_' + secrets.token_hex(16) + ' in the rules array.\n')
        bodies[rule] = body
        (package / 'rule.txt').write_text(body)
        shutil.copyfile(TEMPLATES / 'codex/hook.py', package / 'hook.py')
        write_json(package / 'fixture.json', {'project': str(project), 'rule': rule,
                                             'trace': str(root / 'activation.jsonl')})
        write_json(package / '.codex-plugin/plugin.json',
                   {'name': package.name, 'version': '0.0.0', 'hooks': './hooks/hooks.json'})
        write_json(package / 'hooks/hooks.json', {'hooks': {'SessionStart': [{'hooks': [
            {'type': 'command', 'command': 'python3 "${PLUGIN_ROOT}/hook.py"'}]}]}})
        packages.append(package)
    write_json(root / 'delivery/.agents/plugins/marketplace.json',
               {'name': market, 'plugins': [{'name': p.name, 'source': {
                   'source': 'local', 'path': './packages/' + p.name}} for p in packages]})
    return {'root': root, 'project': project, 'packages': packages, 'rule_bodies': bodies,
            'marketplace': root / 'delivery', 'market_name': market, 'pin': '0.164.0',
            'fallback': 'ABSENT_' + secrets.token_hex(16), 'qualified': False,
            'trust': 'Ordinary user hook review required; never preapproved by fixture'}


def count_developer_rule_bodies(messages, bodies):
    """Count exact nonoverlapping bodies across developer text blocks."""
    if (set(bodies) != {'a', 'b'} or any(not isinstance(b, str) or not b for b in bodies.values())
            or len(set(bodies.values())) != 2):
        raise ValueError('Two distinct nonempty rule bodies are required')
    first, second = bodies.values()
    if first in second or second in first:
        raise ValueError('Rule bodies must not contain one another')
    counts = dict.fromkeys(bodies, 0)
    for message in messages:
        if not isinstance(message, dict):
            raise ValueError('Diagnostic messages must be objects')
        if message.get('type') != 'message' or message.get('role') != 'developer':
            continue
        content = message.get('content', [])
        if not isinstance(content, list) or any(not isinstance(block, dict) for block in content):
            raise ValueError('Developer content must be a list of blocks')
        for block in content:
            if block.get('type') != 'input_text' or not isinstance(block.get('text'), str):
                continue
            for rule, body in bodies.items():
                counts[rule] += block['text'].count(body)
    return counts


def classify_codex_rule_lifecycle(observation):
    """Gate startup, resume and two compactions; clear and fork are not covered."""
    def one_each(counts):
        return (isinstance(counts, dict) and set(counts) == {'a', 'b'}
                and all(type(n) is int and n == 1 for n in counts.values()))

    session = observation.get('session')
    positive = observation.get('positive')
    if (observation.get('positive_passed') is not True or not isinstance(positive, dict)
            or not isinstance(positive.get('session'), str) or not positive['session'].strip()
            or positive.get('completed') is not True or positive.get('read_only') is not True
            or not one_each(positive.get('body_counts'))
            or (isinstance(session, str) and session.strip() and positive['session'] != session)):
        return 'positive_control_failed'
    boundaries = observation.get('boundaries', [])
    if (observation.get('pin') != '0.164.0' or not isinstance(session, str) or not session.strip()
            or not isinstance(boundaries, list) or any(not isinstance(b, dict) for b in boundaries)
            or [b.get('kind') for b in boundaries] != ['resume', 'compact', 'compact']):
        return 'lifecycle_control_failed'
    for boundary in boundaries:
        if (boundary.get('session') != session or boundary.get('completed') is not True
                or boundary.get('read_only') is not True
                or (boundary['kind'] == 'compact' and boundary.get('automatic') is not True)):
            return 'lifecycle_control_failed'
    if any(not one_each(boundary.get('body_counts')) for boundary in boundaries):
        return 'body_count_mismatch'
    if any(b.get('effective_input_observed') is not True for b in boundaries):
        return 'effective_input_unobserved'
    return 'observed_single_copy'


def replay_codex_rule_lifecycle(packet):
    """Replay pinned diagnostic windows without inventing a resume-only result."""
    def load(name):
        return json.loads((packet / name).read_text())

    def same_json(actual, expected):
        return json.dumps(actual, sort_keys=True) == json.dumps(expected, sort_keys=True)

    rows, bodies = load('native-selection.json'), load('rule-bodies.json')
    positive_events, positive_receipt = load('positive-events.json'), load('positive-receipt.json')
    command, exit_receipt, inputs = load('resume-command.json'), load('exit-receipt.json'), load('input-audit.json')
    assessment, summary, audit = load('assessment.json'), load('summary.json'), load('native-window-audit.json')
    if (not isinstance(rows, list) or not rows
            or any(not isinstance(row, dict) or type(row.get('line')) is not int
                   or not isinstance(row.get('payload'), dict) for row in rows)
            or [row['line'] for row in rows] != sorted({row['line'] for row in rows})):
        raise ValueError('Q9 native rows require unique ordered source lines')
    metadata = [row['payload'] for row in rows if row.get('type') == 'session_meta']
    if len(metadata) != 1:
        raise ValueError('Q9 requires one native session identity')
    session, pin = metadata[0].get('id'), metadata[0].get('cli_version')
    if (pin != '0.164.0' or not isinstance(session, str) or not session.strip()
            or positive_receipt.get('pin') != pin or command.get('pin') != pin
            or command.get('session') != session):
        raise ValueError('Q9 native and command pins or sessions disagree')
    for rule in ('a', 'b'):
        expected = [row['sha256'] for row in positive_receipt['before']
                    if row['path'].endswith('/rule.txt') and ('rule-' + rule + '-') in row['path']]
        if (len(expected) != 1 or not isinstance(bodies.get(rule), str)
                or hashlib.sha256(bodies[rule].encode()).hexdigest() != expected[0]):
            raise ValueError('Q9 body differs from its original package receipt')
    argv = command['command']
    if (argv.count('resume') != 1 or argv[argv.index('resume') + 1] != session
            or 'sandbox_mode="read-only"' not in argv
            or 'model_auto_compact_token_limit=1000' not in argv
            or type(exit_receipt.get('native_child_exit_code')) is not int
            or exit_receipt['native_child_exit_code'] != 0
            or inputs.get('no_manual_compaction') is not True
            or len(inputs.get('inputs', [])) != 2
            or any(not isinstance(text, str) or not text.strip() or '/compact' in text for text in inputs['inputs'])):
        raise ValueError('Q9 combined UI control or automatic-compaction evidence is incomplete')
    compacts = [row for row in rows if row.get('type') == 'compacted']
    contexts = [row for row in rows if row.get('type') == 'turn_context']
    if (len(compacts) != 2 or len(contexts) != 3
            or any(row['payload'].get('native_compaction_record_present') is not True for row in compacts)
            or [row['payload'].get('window_number') for row in compacts] != [1, 2]):
        raise ValueError('Q9 requires the two retained native compactions')
    windows = [(rows[0]['line'], compacts[0]['line']),
               (compacts[0]['line'], compacts[1]['line']),
               (compacts[1]['line'], rows[-1]['line'] + 1)]
    counts, turns = [], []
    for start, end in windows:
        window = [row for row in rows if start <= row['line'] < end]
        context = [row for row in window if row.get('type') == 'turn_context']
        if len(context) != 1:
            raise ValueError('Q9 window requires one native turn context')
        context = context[0]
        turn = context['payload'].get('turn_id')
        starts = [row for row in rows if row.get('type') == 'event_msg'
                  and row['payload'].get('type') == 'task_started' and row['payload'].get('turn_id') == turn]
        ends = [row for row in window if row.get('type') == 'event_msg'
                and row['payload'].get('type') == 'task_complete' and row['payload'].get('turn_id') == turn]
        if (not isinstance(turn, str) or not turn.strip() or len(starts) != 1 or len(ends) != 1
                or not starts[0]['line'] < context['line'] < ends[0]['line']
                or context['payload'].get('sandbox_policy') != {'type': 'read-only'}):
            raise ValueError('Q9 native turn identity, completion or read-only policy is incomplete')
        messages = [row['payload'] for row in window if row.get('type') == 'response_item']
        counts.append(count_developer_rule_bodies(messages, bodies))
        turns.append(turn)
    if len(set(turns)) != 3:
        raise ValueError('Q9 requires three distinct completed turns')
    expected_response = positive_receipt['expected_response']
    try:
        positive_completed = (type(positive_receipt.get('exit_code')) is int and positive_receipt['exit_code'] == 0
                              and [row.get('type') for row in positive_events] == ['thread.started', 'item.completed', 'turn.completed']
                              and positive_events[0].get('thread_id') == session
                              and positive_events[1]['item'].get('type') == 'agent_message'
                              and json.loads(positive_events[1]['item']['text']) == expected_response)
    except (KeyError, TypeError, ValueError):
        positive_completed = False
    positive = {'session': session, 'completed': positive_completed, 'read_only': True, 'body_counts': counts[0]}
    boundaries = [{'kind': 'resume', 'session': session, 'completed': False, 'read_only': True,
                   'automatic': False, 'body_counts': None, 'effective_input_observed': False,
                   'control_status': 'NOT RUN separately; the resumed turn immediately compacted'}]
    for index, row in enumerate(compacts):
        boundaries.append({'kind': 'compact', 'index': index + 1, 'native_line': row['line'],
                           'session': session, 'completed': True, 'read_only': True, 'automatic': True,
                           'body_counts': counts[index + 1], 'effective_input_observed': False})
    derived = {'pin': pin, 'session': session, 'positive_passed': positive_completed,
               'positive': positive, 'boundaries': boundaries}
    classification = classify_codex_rule_lifecycle(derived)
    status = 'FAIL: duplicate native developer bodies in the combined reopen/first automatic-compaction window'
    if not any(number > 1 for number in counts[1].values()):
        status = 'INCOMPLETE: separate resume-only control and effective input remain unobserved'
    for key in ('pin', 'session', 'positive_passed', 'positive'):
        if key not in assessment or not same_json(assessment[key], derived[key]):
            raise ValueError('Q9 assessment differs from retained native evidence: ' + key)
    if len(assessment.get('boundaries', [])) != 3:
        raise ValueError('Q9 assessment must name the missing resume-only control')
    for expected, recorded in zip(boundaries, assessment['boundaries']):
        if any(key not in recorded or not same_json(recorded[key], value) for key, value in expected.items()):
            raise ValueError('Q9 recorded boundary differs from native evidence')
    legacy_counts = summary.get('counts')
    if (not isinstance(legacy_counts, dict)
            or not same_json(legacy_counts.get('first_postcompact_rule_bodies'), counts[1])
            or not same_json(legacy_counts.get('second_postcompact_rule_bodies'), counts[2])):
        raise ValueError('Q9 legacy summary body counts differ from native evidence')
    if (assessment.get('qualified') is not False or summary.get('qualified') is not False
            or assessment.get('classification') != classification or assessment.get('status') != status
            or summary.get('status') != status or summary.get('gate_classification') != classification
            or not same_json(summary.get('recorded_body_counts'), counts)
            or not same_json(audit.get('selected_window_counts'), counts) or not same_json(audit.get('raw_window_counts'), counts)
            or audit.get('selected_semantic_sha256') != hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode()).hexdigest()):
        raise ValueError('Q9 classification, count audit or summary differs from replay')
    return {'classification': classification, 'recorded_body_counts': counts}


def prepare_claude_rule_hook(root):
    """Build the Q4 shell-hook package without team guidance or cache writes."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    name = 'upskill-q4-' + secrets.token_hex(8)
    package = root / 'delivery/packages' / name
    marker = 'HOOK_' + secrets.token_hex(16)
    native = 'NATIVE_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    manual = 'MANUAL_' + secrets.token_hex(16)
    trace = root / 'hook-inputs.jsonseq'
    body = 'For the unscoped hook question return exactly ' + marker + ' and nothing else. Do not use tools.\n'
    write_json(package / '.claude-plugin/plugin.json', {'name': name, 'version': '0.0.0', 'description': 'Disposable unscoped rule hook.'})
    write_json(package / 'rules/rule-a.json', {'hookSpecificOutput': {'hookEventName': 'SessionStart', 'additionalContext': body}})
    (package / 'rules/index').write_text('rule-a.json\n')
    (package / 'trace-path').write_text(str(trace) + '\n')
    shutil.copyfile(TEMPLATES / 'claude/hook.sh', package / 'hook.sh')
    write_json(package / 'hooks/hooks.json', {'hooks': {'SessionStart': [{'hooks': [
        {'type': 'command', 'command': 'sh "${CLAUDE_PLUGIN_ROOT}/hook.sh"'}]}]}})
    write_json(root / 'delivery/.claude-plugin/marketplace.json', {'name': name + '-market',
        'owner': {'name': 'Qualification Fixture'}, 'plugins': [{'name': name, 'source': './packages/' + name}]})
    project = root / 'project'
    positive = root / 'positive'
    instructions = positive / '.claude/rules/native-positive.md'
    instructions.parent.mkdir(parents=True)
    instructions.write_text('For the unscoped hook question return exactly ' + native + ' and nothing else. Do not use tools.\n')
    settings = project / '.claude/settings.json'
    write_json(settings, {'env': {'UPSKILL_Q4_MANUAL': manual}})
    settings.chmod(0o640)
    env = fixture_environment(root)
    for path in (project, positive):
        subprocess.run(['git', 'init', '-q', str(path)], env=env, check=True)
        subprocess.run(['git', '-C', str(path), '-c', 'user.name=Qualification Fixture',
                        '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null',
                        '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m',
                        'test: initialize hook fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'package': package, 'marketplace': root / 'delivery', 'project': project,
            'positive_project': positive, 'trace': trace, 'plugin_name': name, 'market_name': name + '-market',
            'hook_marker': marker, 'native_marker': native, 'fallback': fallback, 'pin': '2.1.308'}

def classify_claude_rule_hook(controls, fixture):
    """Classify Q4 callback/response controls without claiming effective copies."""
    def succeeded(row, project):
        return (row.get('pin') == fixture['pin'] and row.get('state') == 'success'
                and row.get('completed') is True and row.get('cwd') == str(project)
                and type(row.get('model_tool_uses')) is int and row['model_tool_uses'] == 0
                and ('native_tools' not in row or row['native_tools'] == []))

    positive = controls.get('native_positive', {})
    if not succeeded(positive, fixture['positive_project']) or positive.get('response') != fixture['native_marker']:
        return 'positive_control_failed'
    installed = controls.get('installed', {})
    if not succeeded(installed, fixture['project']) or installed.get('response') != fixture['hook_marker']:
        return 'installed_positive_failed'
    session = None
    for phase, source in (('installed', 'startup'), ('resume', 'resume'), ('compact', 'compact')):
        row = controls.get(phase, {})
        events = row.get('events', [])
        if (not succeeded(row, fixture['project']) or row.get('response') != fixture['hook_marker']
                or not isinstance(events, list) or len(events) != 1
                or not isinstance(events[0], dict) or events[0].get('source') != source
                or events[0].get('hook_event_name') != 'SessionStart'
                or events[0].get('cwd') != str(fixture['project'])):
            return 'lifecycle_control_failed'
        current = events[0].get('session_id')
        if not isinstance(current, str) or not current.strip() or (session is not None and current != session):
            return 'lifecycle_control_failed'
        session = current
    if controls['compact'].get('native_compaction_completed') is not True:
        return 'lifecycle_control_failed'
    for phase in ('disabled', 'uninstalled'):
        row = controls.get(phase, {})
        if (not succeeded(row, fixture['project']) or row.get('response') != fixture['fallback']
                or row.get('events') != []):
            return 'absence_control_failed'
    return 'observed'


def prepare_opencode_current_routes(root):
    """Prepare current-release Q5 packages without implicit guidance or cache writes."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    project, positive = root / 'project', root / 'positive'
    project.mkdir();positive.mkdir()
    native = 'NATIVE_' + secrets.token_hex(16)
    resource = 'RESOURCE_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    markers = ['RULE_' + secrets.token_hex(16) for _ in range(2)]
    trace = root / 'context.jsonl'
    packages = []
    for rule, marker in zip(('a', 'b'), markers):
        package = root / 'delivery/packages' / ('upskill-q5-' + rule + '-' + secrets.token_hex(8))
        package.mkdir(parents=True)
        (package / 'rule.txt').write_text('For the plugin qualification question include exactly one occurrence of ' + marker + ' in the rules array. Return only the JSON object with a rules array.\n')
        write_json(package / 'fixture.json', {'project': str(project), 'rule': rule, 'trace': str(trace), 'agent': 'build'})
        shutil.copyfile(TEMPLATES / 'opencode/index.ts', package / 'index.ts')
        write_json(package / 'package.json', {'name': package.name, 'version': '0.0.0', 'type': 'module', 'exports': './index.ts', 'dependencies': {'@opencode/plugin': '2.1.4'}})
        packages.append(package)
    source = root / 'delivery/skill-sources'
    skill_id = 'upskill-q5-' + secrets.token_hex(8)
    skill = source / skill_id
    skill.mkdir(parents=True)
    (skill / 'SKILL.md').write_text('---\nname: ' + skill_id + '\ndescription: Disposable explicit package resource procedure.\n---\nRead references/procedure.md relative to this skill and follow it. Do not infer the response without a successful native resource read. If unavailable return exactly ' + fallback + '. Do not edit files or use a shell.\n')
    (skill / 'references').mkdir()
    resource_path = skill / 'references/procedure.md'
    resource_path.write_text('Return exactly ' + resource + ' and no other text.\n')
    deny = {'action': '*', 'resource': '*', 'effect': 'deny'}
    write_json(project / 'opencode.json', {'plugins': [str(p) for p in packages], 'skills': [str(source)], 'permissions': [deny, {'action': 'skill', 'resource': skill_id, 'effect': 'allow'}, {'action': 'read', 'resource': str(resource_path), 'effect': 'allow'}]})
    native_path = positive / 'native-positive.md'
    native_path.write_text('For the native qualification question return exactly ' + native + ' and nothing else. Do not use tools.\n')
    write_json(positive / 'opencode.json', {'plugins': [], 'instructions': [str(native_path)], 'permissions': [deny]})
    env = fixture_environment(root)
    for path in (project, positive):
        subprocess.run(['git', 'init', '-q', str(path)], env=env, check=True)
        subprocess.run(['git', '-C', str(path), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize current routes fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'project': project, 'positive_project': positive, 'packages': packages, 'skill_source': source, 'skill_id': skill_id, 'resource_path': resource_path, 'native_marker': native, 'resource_marker': resource, 'rule_markers': markers, 'fallback': fallback, 'trace': trace, 'pin': '2.1.4'}

def classify_opencode_current_routes(controls, fixture):
    """Keep failed positives, exact callback counts and resource evidence distinct."""
    def succeeded(row):
        return (row.get('pin') == fixture['pin'] and row.get('state') == 'success'
                and row.get('completed') is True)
    positive = controls.get('native_positive', {})
    if not succeeded(positive) or positive.get('response') != fixture['native_marker']:
        return 'positive_control_failed'
    session = None
    for phase in ('context', 'resume'):
        row = controls.get(phase, {})
        response = row.get('response')
        rules = response.get('rules') if isinstance(response, dict) and set(response) == {'rules'} else None
        if (not succeeded(row) or not isinstance(rules, list)
                or any(not isinstance(marker, str) for marker in rules)
                or sorted(rules) != sorted(fixture['rule_markers'])):
            return 'context_control_failed'
        current = row.get('session')
        if not isinstance(current, str) or not current.strip() or (session is not None and current != session):
            return 'context_control_failed'
        session = current
        events = row.get('events', [])
        if (len(events) != 2 or sorted(e.get('rule', '') for e in events) != ['a', 'b']
                or any(e.get('session') != current for e in events)):
            return 'context_control_failed'
    skill = controls.get('skill', {})
    if (not succeeded(skill) or skill.get('response') != fixture['resource_marker']
            or skill.get('native_invocation') is not True or skill.get('resource_read') is not True):
        return 'skill_control_failed'
    verb = controls.get('plugin_add', {})
    if (verb.get('pin') != fixture['pin'] or verb.get('recorded') is not True
            or type(verb.get('exit_code')) is not int):
        return 'install_verb_unrecorded'
    return 'observed'


def prepare(root, client):
    """Create new fixture content without modifying any existing directory."""
    if client not in VERSIONS:
        raise ValueError('Only the pinned Codex/OpenCode candidate fixtures are implemented')
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    root = root.resolve()
    if root.is_relative_to(REPO) or root == Path.home().resolve():
        raise ValueError('Fixture must be outside the repository and personal home root')
    root.mkdir(mode=0o700, parents=True)
    project = root / 'project'
    project.mkdir()
    team = 'TEAM_' + secrets.token_hex(16)
    markers = ['RULE_' + secrets.token_hex(16) for _ in range(2)]
    instructions = project / 'AGENTS.md'
    instructions.write_text('For the plugin qualification question, return a JSON object with '
                            '"team": "' + team + '" and a "rules" array. Include only the '
                            'markers required by automatically loaded plugin rules. Without '
                            'those rules the array is empty. Do not use tools.\n')
    instructions.chmod(0o640)
    env = fixture_environment(root)
    for key in ('HOME', 'CODEX_HOME', 'CLAUDE_CONFIG_DIR', 'COPILOT_HOME',
                'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME'):
        Path(env[key]).mkdir(parents=True, exist_ok=True)
    packages = []
    trace = root / 'activation.jsonl'
    for rule, marker in zip(('a', 'b'), markers):
        package = root / 'marketplace/plugins' / ('upskill-probe-' + rule)
        package.mkdir(parents=True)
        payload = ('For the plugin qualification question include exactly one occurrence of '
                   + marker + ' in the rules array. Preserve the team marker and other rules.\n')
        (package / 'rule.txt').write_text(payload)
        write_json(package / 'fixture.json', {'project': str(project), 'rule': rule,
                                             'trace': str(trace), 'agent': 'build'})
        if client == 'codex':
            shutil.copyfile(TEMPLATES / 'codex/hook.py', package / 'hook.py')
            write_json(package / '.codex-plugin/plugin.json',
                       {'name': 'upskill-probe-' + rule, 'version': '0.0.0', 'hooks': './hooks/hooks.json'})
            write_json(package / 'hooks/hooks.json', {'hooks': {'SessionStart': [{'hooks': [
                {'type': 'command', 'command': 'python3 "${PLUGIN_ROOT}/hook.py"'}]}]}})
        else:
            shutil.copyfile(TEMPLATES / 'opencode/index.ts', package / 'index.ts')
            write_json(package / 'package.json', {'name': 'upskill-probe-' + rule,
                'version': '0.0.0', 'type': 'module', 'exports': './index.ts',
                'dependencies': {'@opencode/plugin': VERSIONS['opencode']}})
        packages.append(str(package))
    if client == 'codex':
        write_json(root / 'marketplace/.agents/plugins/marketplace.json',
            {'name': 'upskill-qualification', 'plugins': [
                {'name': 'upskill-probe-' + rule, 'source': {'source': 'local',
                    'path': './plugins/upskill-probe-' + rule}} for rule in ('a', 'b')]})
    else:
        write_json(project / 'opencode.json', {'plugins': packages,
            'permissions': [{'action': '*', 'resource': '*', 'effect': 'deny'}]})
    fixture = {'root': str(root), 'project': str(project), 'client': client,
               'version': VERSIONS[client], 'team_marker': team, 'rule_markers': markers,
               'team_before': snapshot(instructions), 'packages': packages, 'trace': str(trace),
               'scope': 'project', 'status': 'prepared-only', 'qualified': False,
               'prerequisites': ['native registration/cache roots observed',
                   'normal client-managed authentication in isolated root',
                   'ordinary hook trust for each current definition (Codex)',
                   'successful paired native turns and native activation counts',
                   'remaining platform/scope/reader/lifecycle cells']}
    write_json(root / 'fixture.json', fixture)
    return fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare', action='store_true',
                      help='Create fixtures only; does not run a model or qualify a route')
    mode.add_argument('--replay-codex-rule-lifecycle', type=Path,
                      help='Replay retained Q9 diagnostic windows offline')
    parser.add_argument('--client', choices=VERSIONS)
    parser.add_argument('--root', type=Path, help='New disposable directory outside the repository')
    args = parser.parse_args()
    try:
        if args.replay_codex_rule_lifecycle:
            if args.client or args.root:
                parser.error('Replay cannot be combined with fixture options')
            print(json.dumps(replay_codex_rule_lifecycle(args.replay_codex_rule_lifecycle), indent=2))
        else:
            if not args.client or not args.root:
                parser.error('--prepare requires --client and --root')
            fixture = prepare(args.root, args.client)
            print(Path(fixture['root']) / 'fixture.json')
    except ValueError as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()
