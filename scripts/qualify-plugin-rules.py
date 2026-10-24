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


def prepare_claude_managed_root(root):
    """Build a neutral, copy-only Q6 marketplace without team guidance files."""
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    root = root.resolve()
    parts = root.parts
    forbidden = any(part in ('.claude', '.github', '.codex', '.opencode') for part in parts)
    forbidden = forbidden or any(parts[i:i + 2] in (('.agents', 'skills'), ('.agents', 'plugins')) for i in range(len(parts) - 1))
    if forbidden or root.is_relative_to(REPO) or root == Path.home().resolve() or root.exists():
        raise ValueError('A new neutral fixture root outside client discovery and the repository is required')
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
        return 'invalid_settings'
    if not isinstance(old, dict) or not isinstance(new, dict):
        return 'invalid_settings'
    added = set(new) - set(old)
    permitted = {'enabledPlugins', 'extraKnownMarketplaces'}
    if added - permitted or any(new[key] != {} for key in added):
        return 'preservation_failed'
    remaining = {key: value for key, value in new.items() if key not in added}
    if json.dumps(remaining, sort_keys=True) != json.dumps(old, sort_keys=True):
        return 'preservation_failed'
    return 'empty_object_residue' if added else 'settings_bytes_changed'


def prepare_codex_rule_lifecycle(root):
    """Prepare Q9 without writing team guidance or assuming hook trust."""
    if os.path.lexists(root):
        raise ValueError('Fixture root must not exist')
    root = root.resolve()
    parts = root.parts
    if (root.is_relative_to(REPO) or root == Path.home().resolve()
            or any(p in ('.claude', '.github', '.codex', '.opencode') for p in parts)
            or any(parts[i:i + 2] in (('.agents', 'skills'), ('.agents', 'plugins'))
                   for i in range(len(parts) - 1))):
        raise ValueError('A new neutral fixture root is required')
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
    """Count exact payload occurrences in a selected diagnostic input boundary."""
    if (set(bodies) != {'a', 'b'} or any(not isinstance(b, str) or not b for b in bodies.values())
            or len(set(bodies.values())) != 2):
        raise ValueError('Two distinct nonempty rule bodies are required')
    counts = dict.fromkeys(bodies, 0)
    for message in messages:
        if message.get('type') != 'message' or message.get('role') != 'developer':
            continue
        for block in message.get('content', []):
            if block.get('type') != 'input_text' or not isinstance(block.get('text'), str):
                continue
            for rule, body in bodies.items():
                counts[rule] += block['text'].count(body)
    return counts


def classify_codex_rule_lifecycle(observation):
    """Require all Q9 boundaries; callbacks and responses cannot prove copies."""
    if observation.get('positive_passed') is not True:
        return 'positive_control_failed'
    boundaries = observation.get('boundaries', [])
    session = observation.get('session')
    if (observation.get('pin') != '0.164.0' or not isinstance(session, str) or not session
            or [b.get('kind') for b in boundaries] != ['resume', 'compact', 'compact']):
        return 'lifecycle_control_failed'
    for boundary in boundaries:
        if (boundary.get('session') != session or boundary.get('completed') is not True
                or boundary.get('read_only') is not True
                or (boundary['kind'] == 'compact' and boundary.get('automatic') is not True)):
            return 'lifecycle_control_failed'
        counts = boundary.get('body_counts')
        if (not isinstance(counts, dict) or set(counts) != {'a', 'b'}
                or any(type(n) is not int or n != 1 for n in counts.values())):
            return 'body_count_mismatch'
    if any(b.get('effective_input_observed') is not True for b in boundaries):
        return 'effective_input_unobserved'
    return 'observed_single_copy'


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
    parser.add_argument('--prepare', action='store_true', required=True,
                        help='Create fixtures only; does not run a model or qualify a route')
    parser.add_argument('--client', choices=VERSIONS, required=True)
    parser.add_argument('--root', type=Path, required=True, help='New disposable directory outside the repository')
    args = parser.parse_args()
    try:
        fixture = prepare(args.root, args.client)
    except ValueError as error:
        parser.error(str(error))
    print(Path(fixture['root']) / 'fixture.json')


if __name__ == '__main__':
    main()
