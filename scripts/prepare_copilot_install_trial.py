#!/usr/bin/env python3
"""Prepare the Copilot installation trial packet without running any native client.

The launcher, node and entrypoint are only read as bytes and never executed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import re
import stat
import subprocess
import sys
import tempfile

EXPECTED_FILES = 9
EXPECTED_COMMANDS = 11
# Mirrors upskill::compile::skill_package::is_valid_native_name.
NATIVE_NAME = re.compile(r'upskill-(?!-)(?!.*--)[a-z0-9-]{1,39}(?<!-)-[0-9a-f]{16}')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def mode_of(path):
    return oct(stat.S_IMODE(path.stat().st_mode))


def pin(path):
    resolved = path.resolve()
    data = resolved.read_bytes()
    return {
        'selected': str(path),
        'resolved': str(resolved),
        'sha256': digest(data),
        'bytes': len(data),
        'mode': mode_of(resolved),
    }


def parse_args(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture-binary', required=True)
    parser.add_argument('--required-cli-version', required=True)
    parser.add_argument('--parent-dir', default='/private/tmp')
    # Defaults point at a private install of the pinned client, which cannot
    # auto-update: npm install --prefix PINNED_PREFIX @github/copilot@<version>.
    modules = Path.home() / '.local/share/upskill-trial/copilot-1.0.107/node_modules'
    parser.add_argument('--launcher', default=str(modules / '.bin/copilot'))
    parser.add_argument('--node', default='/opt/homebrew/bin/node')
    parser.add_argument(
        '--entrypoint',
        default=str(modules / '@github/copilot-darwin-arm64/copilot'))
    parser.add_argument(
        '--wrapper-package', default=str(modules / '@github/copilot/package.json'))
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(sys.argv[1:] if argv is None else argv)

    parent = Path(tempfile.mkdtemp(
        prefix='upskill-copilot-install-trial-', dir=args.parent_dir))
    parent.chmod(0o700)
    root = parent / 'packet'
    generator_home = parent / 'generator-home'
    generator_home.mkdir(mode=0o700)
    generator_home.chmod(0o700)

    # The generator gets a minimal environment so nothing leaks from the caller.
    env = {
        'HOME': str(generator_home),
        'USERPROFILE': str(generator_home),
        'PATH': '/usr/bin:/bin',
    }
    try:
        subprocess.run([args.fixture_binary, '--copilot', str(root)],
                       env=env, capture_output=True, check=True)
    except subprocess.CalledProcessError as error:
        sys.stderr.write(
            f'fixture binary failed with exit code {error.returncode}: '
            f'{error.stderr.decode(errors="replace")}\n')
        return error.returncode or 1

    files = []
    for path in root.rglob('*'):
        if path.is_file():
            data = path.read_bytes()
            files.append({
                'path': str(path.relative_to(root)),
                'sha256': digest(data),
                'bytes': len(data),
                'mode': mode_of(path),
            })

    plan_bytes = (root / 'plan.json').read_bytes()
    plan_hash = digest(plan_bytes)
    plan = json.loads(plan_bytes)

    problems = []
    if len(files) != EXPECTED_FILES:
        problems.append(f'expected {EXPECTED_FILES} fixture files, found {len(files)}')
    if len(plan['commands']) != EXPECTED_COMMANDS:
        problems.append(
            f'expected {EXPECTED_COMMANDS} commands, found {len(plan["commands"])}')
    if not NATIVE_NAME.fullmatch(plan['native_name']):
        problems.append(f'native_name {plan["native_name"]!r} is not a valid native name')
    if problems:
        sys.stderr.write('; '.join(problems) + '\n')
        return 1

    wrapper = json.loads(Path(args.wrapper_package).read_bytes())
    prep = {
        'status': 'prepared only; no native command executed',
        'os': platform.system(),
        'machine': platform.machine(),
        'launcher': pin(Path(args.launcher)),
        'node': pin(Path(args.node)),
        'platform_entrypoint': pin(Path(args.entrypoint)),
        'wrapper_package_version': wrapper['version'],
        'required_cli_version': args.required_cli_version,
        'version_mismatch_policy': 'stop before inventories or mutations; no update or fallback',
        'plan_sha256': plan_hash,
        'fixture_files': files,
    }
    prep_path = parent / 'preparation.json'
    prep_bytes = (json.dumps(prep, indent=2) + '\n').encode()
    prep_path.write_bytes(prep_bytes)
    prep_path.chmod(0o600)

    print(json.dumps({
        'packet_root': str(root),
        'plan_sha256': plan_hash,
        'preparation_sha256': digest(prep_bytes),
        'native_name': plan['native_name'],
    }, indent=2))
    return 0


if __name__ == '__main__':
    sys.exit(main())
