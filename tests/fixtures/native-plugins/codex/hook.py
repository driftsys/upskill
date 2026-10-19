#!/usr/bin/env python3
"""Disposable SessionStart probe; no network, tools, credentials or team edits."""
import json
from pathlib import Path
import sys


root = Path(__file__).resolve().parent
config = json.loads((root / 'fixture.json').read_text())
event = json.load(sys.stdin)
if (event.get('hook_event_name') != 'SessionStart'
        or event.get('source') not in ('startup', 'resume', 'clear', 'compact', 'fork')
        or not isinstance(event.get('session_id'), str)
        or not isinstance(event.get('cwd'), str)
        or Path(event['cwd']).resolve() != Path(config['project']).resolve()):
    print('{}')
else:
    record = {'rule': config['rule'], 'session': event['session_id'], 'source': event['source']}
    with Path(config['trace']).open('a') as trace:
        trace.write(json.dumps(record) + '\n')
    print(json.dumps({'hookSpecificOutput': {'hookEventName': 'SessionStart',
        'additionalContext': (root / 'rule.txt').read_text()}}))
