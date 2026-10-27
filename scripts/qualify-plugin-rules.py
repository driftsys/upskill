#!/usr/bin/env python3
"""Prepare disposable native delivery fixtures; offline results never qualify clients."""
import argparse
import datetime
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
    # Preserve OS account identity for native authentication without token overrides.
    env = {k: v for k, v in os.environ.items()
           if k in ('PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'TERM', 'SystemRoot', 'USER')}
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



def prepare_claude_scope_overlap(root, *, pin):
    """Reuse Q6's neutral marketplace for one logical Q10 skill and resource."""
    if (not isinstance(pin, str) or len(pin.split('.')) != 3
            or any(not part.isdigit() for part in pin.split('.'))):
        raise ValueError('An explicit released client pin is required')
    f = prepare_claude_managed_root(root)
    old_package = f['package']
    f['plugin_name'] = f['plugin_name'].replace('upskill-q6-', 'upskill-q10-', 1)
    f['market_name'] = f['market_name'].replace('upskill-q6-market-', 'upskill-q10-market-', 1)
    f['package'] = old_package.with_name(f['plugin_name'])
    old_package.rename(f['package'])
    f['skill'] = f['package'] / 'skills' / f['skill_name'] / 'SKILL.md'
    f['pin'] = pin
    f['resource_marker'] = f.pop('initial_marker')
    f.pop('edited_marker')
    f['activation_marker'] = 'ACTIVATION_' + secrets.token_hex(16)
    f['skill_body'] = ('Activation receipt: ' + f['activation_marker'] + '.\n'
                       'Read references/receipt.txt exactly once, relative to this skill base directory.\n'
                       'Return only the single line read from that file. Do not invoke another skill or use other tools.\n')
    f['resource'] = f['skill'].parent / 'references/receipt.txt'
    f['resource'].parent.mkdir()
    f['resource'].write_text(f['resource_marker'] + '\n')
    f['skill'].write_text('---\nname: ' + f['skill_name'] + '\ndescription: Explicit scope-overlap qualification skill.\n---\n' + f['skill_body'])
    write_json(f['managed_root'] / '.claude-plugin/marketplace.json', {'name': f['market_name'], 'owner': {'name': 'Qualification Fixture'}, 'plugins': [{'name': f['plugin_name'], 'source': './packages/' + f['plugin_name']}]})
    write_json(f['package'] / '.claude-plugin/plugin.json', {'name': f['plugin_name'], 'version': '0.0.0', 'description': 'Disposable Q10 scope overlap fixture.'})
    return f


def inspect_claude_scope_phase(row, fixture, scopes):
    """Inspect a native phase before admitting another scope or counting overlap."""
    import re
    namespace = fixture['plugin_name'] + ':' + fixture['skill_name']
    project = str(fixture['project'])
    if (not isinstance(row, dict) or row.get('exit_code') != 0
            or type(row.get('exit_code')) is not int or row.get('timed_out') is not False
            or row.get('version') != fixture['pin'] or row.get('cwd') != project):
        return 'control_failed', None
    if row.get('malformed_stream') is not False:
        return 'activation_evidence_unobserved', None
    events = row.get('events')
    if not isinstance(events, list) or any(not isinstance(e, dict) for e in events):
        return 'control_failed', None
    init = [e for e in events if e.get('type') == 'system' and e.get('subtype') == 'init']
    results = [e for e in events if e.get('type') == 'result']
    if len(init) != 1 or len(results) != 1:
        return 'control_failed', None
    start, result = init[0], results[0]
    sid = start.get('session_id')
    if (not isinstance(sid, str) or not sid.strip()
            or start.get('claude_code_version') != fixture['pin'] or start.get('cwd') != project
            or any(e.get('session_id') != sid for e in events if e.get('type') in ('assistant', 'user', 'result'))
            or result.get('subtype') != 'success' or result.get('is_error') is not False
            or result.get('permission_denials') != [] or result.get('result') != fixture['resource_marker']):
        return 'control_failed', None
    catalogs = [start.get(key) for key in ['skills', 'slash_commands']]
    if any(not isinstance(items, list) or any(not isinstance(x, str) for x in items) for items in catalogs):
        return 'activation_evidence_unobserved', None
    catalog_counts = [sum(x in (namespace, fixture['skill_name']) for x in items) for items in catalogs]
    calls, replies, texts = [], [], []
    for event in events:
        if event.get('type') not in ('assistant', 'user'):
            continue
        message = event.get('message')
        if not isinstance(message, dict) or not isinstance(message.get('content'), list):
            return 'control_failed', None
        for block in message['content']:
            if not isinstance(block, dict):
                return 'control_failed', None
            if event['type'] == 'assistant' and block.get('type') == 'tool_use':
                calls.append(block)
            if event['type'] == 'user' and block.get('type') == 'tool_result':
                replies.append(block)
            if event['type'] == 'user' and block.get('type') == 'text' and isinstance(block.get('text'), str):
                texts.append(block['text'])
    ids = [c.get('id') for c in calls]
    if (not calls or any(not isinstance(i, str) or not i for i in ids)
            or len(set(ids)) != len(ids) or len(replies) != len(calls)):
        return 'control_failed', None
    allowed = {str(fixture['resource'])}
    bindings = row.get('bindings', [])
    if not isinstance(bindings, list) or any(not isinstance(b, dict) for b in bindings):
        return 'registration_mismatch', None
    settings = row.get('enabled_scopes')
    if (not isinstance(settings, dict) or set(settings) != {'user', 'project'}
            or any(settings[s] is not True if s in scopes else settings[s] is not None and settings[s] is not False for s in settings)
            or any(b.get('scope') not in ('user', 'project') for b in bindings)
            or sorted(b['scope'] for b in bindings) != sorted(scopes)
            or any(b.get('version') != '0.0.0' or (b['scope'] == 'project' and b.get('projectPath') != project) for b in bindings)):
        return 'registration_mismatch', None
    for binding in bindings:
        if isinstance(binding.get('installPath'), str):
            allowed.add(str(Path(binding['installPath']) / 'skills' / fixture['skill_name'] / 'references/receipt.txt'))
    skill_calls = read_calls = 0
    marker_pattern = r'(?<!\w)' + re.escape(fixture['resource_marker']) + r'(?!\w)'
    for call in calls:
        matches = [b for b in replies if b.get('tool_use_id') == call['id']]
        if len(matches) != 1 or ('is_error' in matches[0] and matches[0]['is_error'] is not False):
            return 'control_failed', None
        value = call.get('input')
        if not isinstance(value, dict):
            return 'control_failed', None
        if call.get('name') == 'Skill' and value.get('skill') == namespace:
            skill_calls += 1
        elif call.get('name') == 'Read' and value.get('file_path') in allowed:
            content = matches[0].get('content')
            if not isinstance(content, str) or len(re.findall(marker_pattern, content)) != 1:
                return 'control_failed', None
            read_calls += 1
        else:
            return 'control_failed', None
    if not skill_calls or not read_calls:
        return 'control_failed', None
    bodies = sum(t.count(fixture['skill_body'].strip()) for t in texts)
    markers = sum(t.count(fixture['activation_marker']) for t in texts)
    if not bodies or bodies != markers:
        return 'activation_evidence_unobserved', None
    return None, {'session': sid, 'counts': [*catalog_counts, skill_calls, bodies, read_calls]}



def classify_claude_scope_overlap(controls, fixture):
    """Classify visible catalog/body activations, never provider-effective input."""
    expected = {'project_positive': ['project'], 'user_positive': ['user'],
                'overlap': ['project', 'user'], 'user_remaining': ['user'],
                'project_remaining': ['project']}
    phases = controls.get('phases', {})

    evidence = {}
    for phase in ['project_positive', 'user_positive']:
        reason, row = inspect_claude_scope_phase(phases.get(phase), fixture, expected[phase])
        if reason or row['counts'] != [1, 1, 1, 1, 1]:
            return 'positive_control_failed'
        evidence[phase] = row
    if controls.get('preserved') is not True:
        return 'preservation_failed'
    for phase in ['overlap', 'user_remaining', 'project_remaining']:
        if phase not in phases:
            return 'control_incomplete'
        reason, row = inspect_claude_scope_phase(phases[phase], fixture, expected[phase])
        if reason:
            return reason
        evidence[phase] = row
        if len({e['session'] for e in evidence.values()}) != len(evidence):
            return 'session_identity_mismatch'
        counts = row['counts']
        if any(n > 1 for n in counts[:4]):
            return 'duplicate_exposure_or_activation'
        if counts != [1, 1, 1, 1, 1]:
            return 'control_failed'
    return 'observed_single_entry_and_activation'


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


def classify_claude_inline_bookkeeping(before, after, before_mode, after_mode, *, plugin_name, namespace):
    """Recognize only the measured inline fixture's usage and cache timestamp."""
    if (type(before_mode) is not int or type(after_mode) is not int
            or not 0 <= before_mode <= 0o7777 or before_mode != after_mode
            or not plugin_name or not namespace.startswith(plugin_name + ':')):
        return 'preservation_failed'
    if before == after:
        return 'unchanged'
    try:
        old, new = json.loads(before), json.loads(after)
    except (ValueError, TypeError):
        return 'preservation_failed'
    if not isinstance(old, dict) or not isinstance(new, dict):
        return 'preservation_failed'
    def same(a, b):
        return json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True)
    allowed = {'pluginUsage', 'skillUsage', 'cachedGrowthBookFeaturesAt'}
    if not same({k: v for k, v in old.items() if k not in allowed},
                {k: v for k, v in new.items() if k not in allowed}):
        return 'preservation_failed'
    old_time, new_time = old.get('cachedGrowthBookFeaturesAt'), new.get('cachedGrowthBookFeaturesAt')
    if type(old_time) is not int or type(new_time) is not int or new_time < old_time:
        return 'preservation_failed'
    changed = old_time != new_time
    for field, key, fields in (
            ('pluginUsage', plugin_name + '@inline', {'usageCount', 'lastUsedAt', 'lastUsedNumStartups'}),
            ('skillUsage', namespace, {'usageCount', 'lastUsedAt'})):
        previous, current = old.get(field, {}), new.get(field, {})
        if not isinstance(previous, dict) or not isinstance(current, dict) or key in previous:
            return 'preservation_failed'
        if not same(previous, {k: v for k, v in current.items() if k != key}):
            return 'preservation_failed'
        if key in current:
            entry = current[key]
            if (not isinstance(entry, dict) or set(entry) != fields
                    or any(type(value) is not int or value < 0 for value in entry.values())
                    or entry['usageCount'] < 1):
                return 'preservation_failed'
            changed = True
        elif (field in old) != (field in new):
            return 'preservation_failed'
    return 'native_bookkeeping_residue' if changed else 'preservation_failed'


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



def classify_codex_resume_only(observation):
    """Attribute recorded bodies to resume without claiming effective input."""
    session = observation.get('session')
    def valid_turn(turn):
        return (isinstance(turn, dict) and turn.get('completed') is True
                and turn.get('read_only') is True and turn.get('tools_observed') is False
                and turn.get('response_matches') is True and turn.get('session') == session
                and isinstance(turn.get('turn_id'), str) and bool(turn['turn_id'].strip()))
    positive = observation.get('positive')
    if (not isinstance(session, str) or not session.strip() or not valid_turn(positive)
            or positive.get('body_counts') != {'a': 1, 'b': 1}
            or any(type(n) is not int for n in positive['body_counts'].values())):
        return 'positive_control_failed'
    if observation.get('pin') != '0.164.0' or observation.get('preserved') is not True:
        return 'control_metadata_failed'
    resume = observation.get('resume')
    if not valid_turn(resume) or resume['turn_id'] == positive['turn_id']:
        return 'resume_control_failed'
    counts = resume.get('body_counts')
    if (type(resume.get('compaction_records')) is not int or resume['compaction_records'] != 0
            or resume.get('callback_sources') != ['resume', 'resume']
            or not isinstance(counts, dict) or set(counts) != {'a', 'b'}
            or any(type(n) is not int or n < 0 for n in counts.values())):
        return 'resume_control_failed'
    return 'recorded_counts_observed'


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


def prepare_codex_skill_updates(root):
    """Prepare one package skill and independent source-edit response markers."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    name = 'upskill-q8-' + secrets.token_hex(8)
    skill_name = 'procedure-' + secrets.token_hex(8)
    initial = 'INITIAL_' + secrets.token_hex(16)
    edited = 'EDITED_' + secrets.token_hex(16)
    fallback = 'ABSENT_' + secrets.token_hex(16)
    package = root / 'delivery/plugins' / name
    write_json(package / '.codex-plugin/plugin.json', {'name': name, 'version': '0.0.0', 'description': 'Disposable skill update qualification.', 'skills': './skills/'})
    skill = package / 'skills' / skill_name
    skill.mkdir(parents=True)
    entry = skill / 'SKILL.md'
    entry.write_text('---\nname: ' + skill_name + '\ndescription: Disposable native plugin resource procedure.\n---\nRead references/procedure.md relative to this skill and follow it. Do not infer its contents or use another source. If unavailable return exactly ' + fallback + '. Do not edit files, delegate or change configuration.\n')
    resource = skill / 'references/procedure.md'
    resource.parent.mkdir()
    resource.write_text('Return exactly ' + initial + ' and nothing else.\n')
    marketplace = root / 'delivery'
    write_json(marketplace / '.agents/plugins/marketplace.json', {'name': name + '-market', 'plugins': [{'name': name, 'source': {'source': 'local', 'path': './plugins/' + name}}]})
    project, unrelated = root / 'project', root / 'unrelated'
    env = fixture_environment(root)
    for path in (project, unrelated):
        path.mkdir()
        subprocess.run(['git', 'init', '-q', str(path)], env=env, check=True)
        subprocess.run(['git', '-C', str(path), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize skill update fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'project': project, 'unrelated_project': unrelated, 'package': package, 'marketplace': marketplace, 'plugin_name': name, 'market_name': name + '-market', 'skill_name': skill_name, 'skill_file': entry, 'resource': resource, 'initial_marker': initial, 'edited_marker': edited, 'fallback': fallback, 'pin': '0.164.0'}


def classify_codex_skill_updates(controls, fixture):
    """Record source edit visibility only after native expansion/resource positives."""
    def valid(row):
        return (row.get('pin') == fixture['pin'] and row.get('state') == 'success'
                and row.get('completed') is True and row.get('native_expansion') is True
                and row.get('resource_read') is True
                and isinstance(row.get('native_identity'), str) and bool(row['native_identity'])
                and isinstance(row.get('load_path'), str) and bool(row['load_path']))
    installed = controls.get('installed', {})
    if not valid(installed) or installed.get('response') != fixture['initial_marker']:
        return 'positive_control_failed'
    for phase in ('edited_without_reinstall', 'reinstalled'):
        row = controls.get(phase, {})
        if not valid(row):
            return 'lifecycle_control_failed'
        if row['native_identity'] != installed['native_identity']:
            return 'identity_mismatch'
    edited = controls['edited_without_reinstall'].get('response')
    if edited not in (fixture['initial_marker'], fixture['edited_marker']):
        return 'edit_response_mismatch'
    if controls['reinstalled'].get('response') != fixture['edited_marker']:
        return 'refresh_control_failed'
    return 'edits_require_refresh' if edited == fixture['initial_marker'] else 'edits_visible_without_refresh'


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




def prepare_codex_agent_registration(root):
    """Prepare Q7 explicit role registration and a plugin-contained TOML copy."""
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    project, positive = root / 'project', root / 'positive'
    project.mkdir();positive.mkdir()
    name = 'upskill-q7-' + secrets.token_hex(8)
    marker = 'CHILD_' + secrets.token_hex(16)
    native = 'NATIVE_' + secrets.token_hex(16)
    resource, forbidden = project / 'resource.txt', project / 'denied.txt'
    resource.write_text(marker + '\n')
    agent = root / 'delivery/agent.toml'
    agent.parent.mkdir(parents=True)
    instructions = ('Read ' + str(resource) + ' using a native tool. Then attempt exactly once to create '
        + str(forbidden) + ' containing the word probe using a native command. Do not request escalation, retry, delegate or change permissions. '
        'The parent is read-only. If the native operation is denied, return only a JSON object with marker equal to the resource contents and write equal to denied. '
        'If it succeeds, report write equal to allowed. Do not infer a denial without the actual attempt.')
    body = 'name = ' + json.dumps(name) + '\ndescription = "Disposable inherited-boundary agent."\ndeveloper_instructions = ' + json.dumps(instructions) + '\n'
    agent.write_text(body)
    package = root / 'delivery/plugins' / name
    write_json(package / '.codex-plugin/plugin.json', {'name': name, 'version': '0.0.0', 'description': 'Disposable plugin-only agent discovery.'})
    (package / 'agents').mkdir()
    (package / 'agents/probe.toml').write_text(body)
    marketplace = root / 'delivery'
    write_json(marketplace / '.agents/plugins/marketplace.json', {'name': name + '-market', 'plugins': [{'name': name, 'source': {'source': 'local', 'path': './plugins/' + name}}]})
    (project / '.codex').mkdir()
    (project / '.codex/config.toml').write_text('[agents]\nenabled = true\nmax_threads = 1\n\n[agents.' + name + ']\ndescription = "Explicit disposable child boundary control."\nconfig_file = ' + json.dumps(str(agent)) + '\n')
    (positive / '.codex').mkdir()
    (positive / '.codex/config.toml').write_text('developer_instructions = ' + json.dumps('For the native qualification question return exactly ' + native + ' and nothing else. Do not use tools.') + '\n')
    env = fixture_environment(root)
    for path in (project, positive):
        subprocess.run(['git', 'init', '-q', str(path)], env=env, check=True)
        subprocess.run(['git', '-C', str(path), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize agent registration fixture ' + secrets.token_hex(16)], env=env, check=True)
    return {'root': root, 'project': project, 'positive_project': positive, 'agent_name': name, 'agent_file': agent, 'package': package, 'plugin_name': name, 'marketplace': marketplace, 'market_name': name + '-market', 'marker': marker, 'native_marker': native, 'resource': resource, 'forbidden_file': forbidden, 'pin': '0.164.0'}



def classify_codex_agent_registration(controls, fixture):
    """Require native delegated boundary evidence before accepting discovery absence."""
    def succeeded(row):
        return (row.get('pin') == fixture['pin'] and row.get('state') == 'success'
                and row.get('completed') is True)
    positive = controls.get('native_positive', {})
    if not succeeded(positive) or positive.get('response') != fixture['native_marker']:
        return 'positive_control_failed'
    child = controls.get('registered', {})
    if (not succeeded(child) or child.get('response') != {'marker': fixture['marker'], 'write': 'denied'}
            or any(child.get(k) is not True for k in ('delegated', 'child_resource_read', 'forbidden_attempted', 'native_denial', 'forbidden_file_absent'))
            or child.get('parent_sandbox') != 'read-only' or child.get('child_sandbox') != 'read-only'):
        return 'child_boundary_control_failed'
    plugin = controls.get('plugin_only', {})
    if (plugin.get('pin') != fixture['pin']
            or any(plugin.get(k) is not True for k in ('native_catalog_observed', 'plugin_installed', 'external_registration_absent'))
            or plugin.get('candidate_present') is not False):
        return 'plugin_discovery_control_failed'
    return 'observed'




def codex_resume_body_windows(startup, resumed, bodies):
    """Keep cumulative native history distinct from newly appended records."""
    if (not isinstance(startup, list) or not startup or not isinstance(resumed, list)
            or any(not isinstance(row, dict) for row in startup + resumed)
            or resumed[:len(startup)] != startup):
        raise ValueError('Unchanged nonempty startup history must prefix the resumed history')
    if any(row.get('type') == 'compacted' for row in resumed):
        raise ValueError('Resume-only windows require no compaction record')
    def count(rows):
        messages = [row.get('payload') for row in rows if row.get('type') == 'response_item']
        return count_developer_rule_bodies(messages, bodies)
    return {'startup': count(startup), 'resumed_history': count(resumed),
            'resume_new_records': count(resumed[len(startup):])}


def cross_client_skill_tree(package):
    """Capture every skill file and mode without following symbolic links."""
    skills = package / 'skills'
    if package.is_symlink() or skills.is_symlink() or not skills.is_dir():
        raise ValueError('A regular skill directory is required')
    tree = {}
    for path in sorted(skills.rglob('*')):
        if path.is_symlink():
            raise ValueError('Symbolic links are not permitted in qualification skills')
        if path.is_file():
            tree[str(path.relative_to(skills))] = snapshot(path)
        elif not path.is_dir():
            raise ValueError('Only regular skill files and directories are permitted')
    if not tree:
        raise ValueError('At least one skill file is required')
    return tree


def cross_client_package_tree(package):
    """Fingerprint the complete envelope separately from portable skill equality."""
    if package.is_symlink() or not package.is_dir():
        raise ValueError('A regular package directory is required')
    tree = {}
    for path in sorted(package.rglob('*')):
        if path.is_symlink():
            raise ValueError('Package symbolic links are not permitted')
        if path.is_file():
            tree[str(path.relative_to(package))] = snapshot(path)
        elif not path.is_dir():
            raise ValueError('Only regular package files and directories are permitted')
    return tree


def prepare_cross_client_isolation(root, *, pins):
    """Prepare paired Q11 session-selector packages with identical skill trees."""
    clients = ('claude', 'copilot', 'codex', 'opencode')
    if any(not isinstance(pins.get(client), str) or not pins[client] for client in clients):
        raise ValueError('Fresh four-client campaign pins are required')
    root = neutral_fixture_root(root)
    root.mkdir(parents=True, mode=0o700)
    env = fixture_environment(root)
    targets = {}
    for client in ('claude', 'copilot'):
        name = 'upskill-q11-' + secrets.token_hex(8)
        skill = 'probe-' + secrets.token_hex(8)
        marker = 'PRESENT_' + secrets.token_hex(16)
        absent = 'ABSENT_' + secrets.token_hex(16)
        packages = {}
        for kind, intended in [('native', client), ('foreign', 'copilot' if client == 'claude' else 'claude')]:
            package = root / client / 'delivery' / kind / name
            manifest = {'name': name, 'version': '0.0.0', 'description': 'Disposable cross-client qualification package.'}
            if intended == 'claude':
                write_json(package / '.claude-plugin/plugin.json', manifest)
            else:
                manifest['$schema'] = 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json'
                write_json(package / 'plugin.json', manifest)
            directory = package / 'skills' / skill
            directory.mkdir(parents=True)
            (directory / 'SKILL.md').write_text('---\nname: ' + skill + '\ndescription: Explicit cross-client qualification probe.\n---\nReturn exactly ' + marker + ' and nothing else. Do not use any other tools.\n')
            (directory / 'resource.txt').write_text('Qualification resource: ' + marker + '\n')
            for file in directory.iterdir():
                file.chmod(0o644)
            packages[kind] = package
        projects = [root / client / name for name in ('positive-project', 'candidate-project')]
        for project in projects:
            project.mkdir(parents=True)
            subprocess.run(['git', 'init', '-q', str(project)], env=env, check=True)
            subprocess.run(['git', '-C', str(project), '-c', 'user.name=Qualification Fixture', '-c', 'user.email=qualification@example.invalid', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '--allow-empty', '-m', 'test: initialize isolation fixture ' + secrets.token_hex(16)], env=env, check=True)
        targets[client] = {'client': client, 'pin': pins[client], 'scope': 'session',
                           'plugin_name': name, 'skill_name': skill, 'namespace': name + ':' + skill,
                           'marker': marker, 'absent_marker': absent,
                           'native_package': packages['native'], 'foreign_package': packages['foreign'],
                           'native_intended': client, 'foreign_intended': 'copilot' if client == 'claude' else 'claude',
                           'native_manifest': packages['native'] / ('.claude-plugin/plugin.json' if client == 'claude' else 'plugin.json'),
                           'foreign_manifest': packages['foreign'] / ('plugin.json' if client == 'claude' else '.claude-plugin/plugin.json'),
                           'positive_project': projects[0], 'candidate_project': projects[1],
                           'packages': {kind: cross_client_package_tree(package) for kind, package in packages.items()},
                           'trees': {kind: cross_client_skill_tree(package) for kind, package in packages.items()}}
    return {'root': root, 'pins': dict(pins), 'targets': targets}


def inspect_cross_client_positive(events, fixture, *, exit_code, timed_out=False, malformed_stream=False):
    """Replay selected native records; a bare-name fallback is not a namespace PASS."""
    namespace = fixture['namespace']
    native = str(fixture['native_package'])
    row = {'pin': fixture['pin'], 'platform': 'macOS arm64',
           'cwd': str(fixture['positive_project']), 'scope': fixture['scope'],
           'namespace': namespace, 'catalog_complete': False, 'catalog_present': False,
           'activations': 0, 'content_activations': 0, 'unexpected_tools': False, 'source_path': None, 'response': None}
    attempts, outcomes = 0, []
    if not isinstance(events, list) or any(not isinstance(e, dict) for e in events):
        events = []
        malformed_stream = True
    results = [event for event in events if event.get('type') == 'result']
    complete = (len(results) == 1 and events[-1] == results[0]
                and exit_code == 0 and not timed_out and not malformed_stream)
    result = results[0] if len(results) == 1 else {}
    if fixture['client'] == 'claude':
        catalogs = [e for e in events if e.get('type') == 'system' and e.get('subtype') == 'init']
        if len(catalogs) == 1:
            catalog = catalogs[0]
            row['catalog_complete'] = isinstance(catalog.get('slash_commands'), list)
            row['catalog_present'] = namespace in catalog.get('slash_commands', [])
            if any(p.get('name') == fixture['plugin_name'] and p.get('path') == native
                   for p in catalog.get('plugins', [])):
                row['source_path'] = native
        starts = [block for e in events if e.get('type') == 'assistant'
                  for block in e.get('message', {}).get('content', [])
                  if isinstance(block, dict) and block.get('type') == 'tool_use']
        ends = [block for e in events if e.get('type') == 'user'
                for block in e.get('message', {}).get('content', [])
                if isinstance(block, dict) and block.get('type') == 'tool_result']
        for start in starts:
            exact = (isinstance(start.get('id'), str) and bool(start['id'])
                     and start.get('name') == 'Skill' and start.get('input', {}).get('skill') == namespace)
            row['unexpected_tools'] |= not exact
            successes = [end for end in ends if end.get('tool_use_id') == start.get('id')
                         and end.get('is_error', False) is False]
            row['activations'] += int(exact and len(successes) == 1)
            related = start.get('input', {}).get('skill') in (namespace, fixture['skill_name'])
            row['content_activations'] += int(related and bool(start.get('id')) and len(successes) == 1)
            if exact:
                attempts += 1
                matches = [end for end in ends if end.get('tool_use_id') == start['id']]
                if len(matches) == 1:
                    end = matches[0]
                    missing = 'Unknown skill: ' + namespace in str(end.get('content', ''))
                    outcomes.append('loaded' if end.get('is_error', False) is False else 'not_found' if missing else 'failed')
        complete &= (result.get('is_error') is False and result.get('subtype') == 'success'
                     and result.get('permission_denials') == [])
        if complete:
            row['response'] = result.get('result')
    else:
        catalogs = [e.get('data', {}).get('skills') for e in events if e.get('type') == 'session.skills_loaded']
        row['catalog_complete'] = len(catalogs) == 1 and isinstance(catalogs[0], list)
        if row['catalog_complete']:
            entries = [skill for skill in catalogs[0] if skill.get('commandName') == namespace and skill.get('enabled') is True]
            row['catalog_present'] = bool(entries)
            expected_path = str(Path(native) / 'skills' / fixture['skill_name'] / 'SKILL.md')
            if len(entries) == 1 and entries[0].get('path') == expected_path:
                row['source_path'] = native
        starts = [e.get('data', {}) for e in events if e.get('type') == 'tool.execution_start']
        ends = [e.get('data', {}) for e in events if e.get('type') == 'tool.execution_complete']
        for start in starts:
            exact = (isinstance(start.get('toolCallId'), str) and bool(start['toolCallId'])
                     and start.get('toolName') == 'skill' and start.get('arguments', {}).get('skill') == namespace)
            row['unexpected_tools'] |= not exact
            successes = [end for end in ends if end.get('toolCallId') == start.get('toolCallId') and end.get('success') is True]
            row['activations'] += int(exact and len(successes) == 1)
            related = start.get('arguments', {}).get('skill') in (namespace, fixture['skill_name'])
            row['content_activations'] += int(related and bool(start.get('toolCallId')) and len(successes) == 1)
            if exact:
                attempts += 1
                matches = [end for end in ends if end.get('toolCallId') == start['toolCallId']]
                if len(matches) == 1:
                    end = matches[0]
                    missing = end.get('error', {}).get('message') == 'Skill not found: ' + namespace
                    outcomes.append('loaded' if end.get('success') is True else 'not_found' if missing else 'failed')
        complete &= type(result.get('exitCode')) is int and result['exitCode'] == 0
        if complete:
            row['response'] = next((e.get('data', {}).get('content') for e in reversed(events)
                                    if e.get('type') == 'assistant.message'), None)
    outcome = outcomes[0] if attempts == 1 and len(outcomes) == 1 else 'failed' if attempts else 'not_attempted'
    row.update(state='success' if complete else 'error', complete=bool(complete),
               invocation_attempted=attempts > 0, invocation_outcome=outcome,
               invocation_complete=bool(complete and outcome in ('loaded', 'not_found')))
    return row


def classify_cross_client_isolation(observation, fixture):
    """Keep bounded isolation and measured preservation as independent results."""
    def receipts_valid(receipts):
        if not isinstance(receipts, dict) or not receipts:
            return False
        for row in receipts.values():
            if not isinstance(row, dict) or type(row.get('present')) is not bool:
                return False
            if row['present']:
                digest = row.get('sha256')
                if (not isinstance(digest, str) or len(digest) != 64
                        or any(c not in '0123456789abcdef' for c in digest)
                        or type(row.get('mode')) is not int or not 0 <= row['mode'] <= 0o7777):
                    return False
            elif row.get('sha256') is not None or row.get('mode') is not None:
                return False
        return True
    preservation = observation.get('preservation', {})
    before, after = preservation.get('before'), preservation.get('after')
    preserved = receipts_valid(before) and receipts_valid(after) and before == after
    def result(isolation):
        return {'isolation': isolation, 'preservation': 'unchanged' if preserved else 'preservation_failed'}
    phases = {key: observation.get(key, {}) for key in ('positive', 'baseline', 'candidate')}
    for name, phase in phases.items():
        project = fixture['positive_project'] if name == 'positive' else fixture['candidate_project']
        if phase and any(phase.get(k) != v for k, v in {
                'pin': fixture['pin'], 'platform': 'macOS arm64', 'cwd': str(project),
                'scope': fixture['scope'], 'namespace': fixture['namespace']}.items()):
            return result('control_metadata_failed')
    trees = fixture['trees']
    if (not trees.get('native') or trees.get('native') != trees.get('foreign')
            or observation.get('trees_after') != trees):
        return result('invalid_pair')
    def completed(phase):
        return (phase.get('state') == 'success'
                and all(phase.get(k) is True for k in ('complete', 'catalog_complete', 'invocation_complete'))
                and type(phase.get('activations')) is int and phase['activations'] >= 0
                and phase.get('unexpected_tools') is False)
    positive, baseline, candidate = (phases[key] for key in ('positive', 'baseline', 'candidate'))
    if (not completed(positive) or positive.get('catalog_present') is not True
            or positive.get('response') != fixture['marker'] or positive.get('activations') != 1
            or positive.get('source_path') != str(fixture['native_package'])
            or positive.get('invocation_attempted') is not True or positive.get('invocation_outcome') != 'loaded'):
        return result('blocked_positive')
    packages = fixture.get('packages')
    if (not isinstance(packages, dict) or set(packages) != {'native', 'foreign'}
            or any(not tree for tree in packages.values()) or observation.get('packages_after') != packages):
        return result('invalid_pair')
    selector = observation.get('selector', {})
    def activated(phase):
        return (any(type(phase.get(key)) is int and phase[key] > 0 for key in ('activations', 'content_activations'))
                or isinstance(phase.get('response'), str) and fixture['marker'] in phase['response'])
    if baseline.get('catalog_present') is True or activated(baseline):
        return result('baseline_contaminated')
    if (not completed(baseline) or baseline.get('catalog_present') is not False
            or baseline.get('response') != fixture['absent_marker'] or baseline.get('activations') != 0
            or baseline.get('invocation_attempted') is not True or baseline.get('invocation_outcome') != 'not_found'):
        return result('blocked_baseline')
    if activated(candidate) or selector.get('status') in ('installed', 'converted'):
        return result('activation_observed')
    if candidate.get('catalog_present') is True:
        return result('catalog_exposure')
    if selector.get('available') is False and selector.get('status') == 'unavailable':
        return result('selector_unavailable')
    if (selector.get('available') is not True or selector.get('completed') is not True
            or selector.get('status') not in ('selected', 'rejected')
            or not completed(candidate) or candidate.get('catalog_present') is not False
            or candidate.get('invocation_attempted') is not True or candidate.get('invocation_outcome') != 'not_found'
            or candidate.get('activations') != 0 or candidate.get('response') != fixture['absent_marker']):
        return result('incomplete_evidence')
    return result('isolation_observed')


def classify_registration_isolation(observation, fixture):
    """Classify one Q11b cell from captured evidence, without native effects.

    Producers must correlate native records into phase rows, including session
    binding and marker disclosure. This classifier does not collect inventory,
    authorize a profile, or replace the explicit-selector Q11 control.
    """
    def mapping(value):
        return value if isinstance(value, dict) else {}

    if not isinstance(observation, dict):
        return {'isolation': 'invalid_capture', 'preservation': 'preservation_failed'}
    if (not isinstance(fixture, dict)
            or any(not isinstance(fixture.get(key), str) or not fixture[key]
                   for key in ('owner', 'observer', 'scope', 'cwd', 'namespace', 'marker', 'absent_marker'))
            or fixture['owner'] == fixture['observer']
            or not isinstance(fixture.get('pins'), dict)
            or any(not isinstance(fixture['pins'].get(client), str) or not fixture['pins'][client]
                   for client in (fixture['owner'], fixture['observer']))
            or not isinstance(fixture.get('source_paths'), dict)
            or not isinstance(fixture.get('aliases'), list)
            or any(not isinstance(alias, str) or not alias for alias in fixture['aliases'])
            or len(set(fixture['aliases'])) != len(fixture['aliases'])):
        return {'isolation': 'invalid_fixture', 'preservation': 'preservation_failed'}

    def receipt_valid(row):
        if not isinstance(row, dict) or type(row.get('present')) is not bool:
            return False
        if not row['present']:
            return row.get('sha256') is None and row.get('mode') is None
        digest = row.get('sha256')
        return (isinstance(digest, str) and len(digest) == 64
                and all(c in '0123456789abcdef' for c in digest)
                and type(row.get('mode')) is int and 0 <= row['mode'] <= 0o7777)

    preservation = mapping(observation.get('preservation'))
    before, after = preservation.get('before'), preservation.get('after')
    protected = fixture.get('protected_paths')
    protected_valid = (isinstance(protected, list) and bool(protected)
                       and all(isinstance(path, str) and bool(path) for path in protected)
                       and len(set(protected)) == len(protected))
    preserved = (protected_valid and isinstance(before, dict) and before == after
                 and set(before) == set(protected)
                 and all(receipt_valid(row) for row in before.values()))

    def result(verdict):
        return {'isolation': verdict, 'preservation': 'unchanged' if preserved else 'preservation_failed'}

    owner, observer = fixture['owner'], fixture['observer']
    inventory = mapping(observation.get('inventory'))
    if (inventory.get('complete') is not True or inventory.get('blocking_reasons') != []
            or any(inventory.get(key) != fixture[key] for key in ('owner', 'observer', 'cwd', 'scope', 'pins'))
            or any(not isinstance(inventory.get(key), list) or not inventory[key]
                   or any(not isinstance(value, str) or not value for value in inventory[key])
                   for key in ('effective_roots', 'reader_conditions', 'surfaces'))):
        return result('blocked_inventory')
    packages, trees = fixture.get('packages'), fixture.get('skill_trees')
    def tree_valid(tree):
        return (isinstance(tree, dict) and bool(tree)
                and all(isinstance(path, str) and bool(path)
                        and not any(part in ('', '.', '..') for part in path.split('/'))
                        and isinstance(row, dict) and set(row) == {'sha256', 'mode'}
                        and receipt_valid(dict(row, present=True)) for path, row in tree.items()))

    if (not isinstance(packages, dict) or set(packages) != {owner, observer}
            or not all(tree_valid(tree) for tree in packages.values())
            or not isinstance(trees, dict) or set(trees) != {owner, observer}
            or not all(tree_valid(tree) for tree in trees.values()) or trees[owner] != trees[observer]
            or any({path[len('skills/'):]: row for path, row in packages[client].items()
                    if path.startswith('skills/')} != trees[client] for client in (owner, observer))
            or observation.get('packages_after') != packages
            or observation.get('skill_trees_after') != trees):
        return result('invalid_pair')
    names = ('observer_positive', 'owner_positive', 'baseline', 'owner_candidate', 'candidate', 'post_removal')
    phases = {name: mapping(mapping(observation.get('phases')).get(name)) for name in names}
    def metadata_valid(name):
        phase = phases[name]
        client = owner if name in ('owner_positive', 'owner_candidate') else observer
        return not phase or all(phase.get(key) == value for key, value in {
                'client': client, 'pin': fixture['pins'][client], 'cwd': fixture['cwd'],
                'scope': 'session' if name == 'observer_positive' else fixture['scope'],
                'namespace': fixture['namespace']}.items())

    for name in names[:-1]:
        if not metadata_valid(name):
            return result('control_metadata_failed')

    def complete(phase):
        return (phase.get('state') == 'success'
                and all(phase.get(key) is True for key in
                        ('complete', 'catalog_complete', 'invocation_complete', 'invocation_attempted'))
                and phase.get('unexpected_tools') is False
                and all(type(phase.get(key)) is int and phase[key] >= 0
                        for key in ('activations', 'content_activations')))

    for name in ('observer_positive', 'owner_positive', 'owner_candidate'):
        phase = phases.get(name, {})
        if (not complete(phase) or phase.get('session_bound') is not True
                or name != 'observer_positive' and phase.get('candidate_registered') is not True
                or phase.get('marker_disclosed') is not False
                or phase.get('catalog_present') is not True
                or phase.get('invocation_outcome') != 'loaded' or phase.get('activations') != 1
                or phase.get('content_activations') != 1
                or not isinstance(fixture.get('source_paths', {}).get(name), str)
                or not fixture['source_paths'][name]
                or phase.get('source_path') != fixture['source_paths'][name]
                or phase.get('response') != fixture['marker']):
            return result('blocked_' + name)

    def interval(row):
        try:
            start = datetime.datetime.fromisoformat(row['started_at'])
            finish = datetime.datetime.fromisoformat(row['finished_at'])
            if start.utcoffset() is None or finish.utcoffset() is None or start >= finish:
                return None
            return start, finish
        except (KeyError, TypeError, ValueError):
            return None

    times, sessions = {}, set()
    def fresh(name):
        phase = phases[name]
        session = phase.get('session_id')
        times[name] = interval(phase)
        if (not times[name] or not isinstance(session, str) or not session
                or session in sessions or phase.get('session_bound') is not True):
            return False
        sessions.add(session)
        return True

    for name in names[:-1]:
        if not fresh(name):
            return result('blocked_freshness')
    operations = mapping(observation.get('operations'))
    def operation_valid(name, preceding):
        operation = mapping(operations.get(name))
        timing = interval(operation)
        if (not timing or operation.get('state') != 'success' or operation.get('complete') is not True
                or any(operation.get(key) != fixture[key] for key in ('owner', 'cwd', 'scope', 'namespace'))
                or timing[0] <= max(times[phase][1] for phase in preceding)):
            return False
        times[name] = timing
        return True

    for name, preceding in [('calibration_cleanup', ('observer_positive', 'owner_positive')),
                            ('owner_install', ('baseline',))]:
        if not operation_valid(name, preceding):
            return result('blocked_operations')
    if (times['baseline'][0] <= times['calibration_cleanup'][1]
            or times['owner_candidate'][0] <= times['owner_install'][1]
            or times['candidate'][0] <= times['owner_candidate'][1]):
        return result('blocked_freshness')
    negatives = [phases[name] for name in ('baseline', 'candidate', 'post_removal')]
    if any(any(phase.get(key) is not False for key in
               ('foreign_selector', 'candidate_registered', 'marker_disclosed')) for phase in negatives[:2]):
        return result('invalid_observer_conditions')

    def activated(phase):
        return (any(type(phase.get(key)) is int and phase[key] > 0
                    for key in ('activations', 'content_activations'))
                or phase.get('alias_activation') is True
                or isinstance(phase.get('response'), str) and fixture['marker'] in phase['response'])

    def absent(phase):
        return (complete(phase) and phase.get('catalog_present') is False
                and phase.get('invocation_outcome') == 'not_found'
                and phase.get('response') == fixture['absent_marker']
                and phase.get('activations') == 0 and phase.get('content_activations') == 0
                and phase.get('alias_activation') is False and phase.get('alias_exposure') is False
                and phase.get('alias_not_found') is True
                and phase.get('aliases_checked') == fixture['aliases'])

    baseline, candidate, removed = negatives
    if activated(baseline) or baseline.get('catalog_present') is True or baseline.get('alias_exposure') is True:
        return result('baseline_contaminated')
    if not absent(baseline):
        return result('blocked_baseline')
    # A correlated activation is a retained counterexample even if cleanup fails.
    if activated(candidate):
        return result('activation_observed')
    if not metadata_valid('post_removal'):
        return result('control_metadata_failed')
    if not fresh('post_removal'):
        return result('blocked_freshness')
    if not operation_valid('owner_remove', ('owner_candidate', 'candidate')):
        return result('blocked_operations')
    if times['post_removal'][0] <= times['owner_remove'][1]:
        return result('blocked_freshness')
    if any(removed.get(key) is not False for key in
           ('foreign_selector', 'candidate_registered', 'marker_disclosed')):
        return result('invalid_observer_conditions')
    if activated(removed):
        return result('activation_observed')
    if any(phase.get('catalog_present') is True or phase.get('alias_exposure') is True
           for phase in (candidate, removed)):
        return result('catalog_exposure')
    if not all(absent(phase) for phase in (candidate, removed)):
        return result('incomplete_evidence')
    return result('discovery_absence_observed')


if __name__ == '__main__':
    main()
