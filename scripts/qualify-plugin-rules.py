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
    requested = root.expanduser().absolute()
    root = requested.resolve()
    if root.exists():
        raise ValueError('Fixture root must not exist')
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
    guidance = ('CLAUDE.md', 'AGENTS.md', *native_roots)
    forbidden |= any((parent / name).exists() or (parent / name).is_symlink()
                     for parent in root.parents for name in guidance)
    if forbidden:
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
