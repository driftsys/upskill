"""Offline native event gates and retained Q1 classification replay."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('q1_replay', ROOT / 'scripts/qualify-native-instructions.py')
q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(q)
PACKET = ROOT / 'docs/technotes/evidence/2026-10-24-copilot-claude-rules-reader'


def events(response='marker', exit_code=0):
    return [
        {'type': 'assistant.message', 'data': {'content': response}},
        {'type': 'result', 'sessionId': 'native-session', 'exitCode': exit_code},
    ]


def encode(rows):
    return '\n'.join(json.dumps(row) for row in rows)


class CopilotReaderReplayTests(unittest.TestCase):
    def test_native_copilot_command_requests_structured_events(self):
        command = q.command('copilot', 'question', Path('unused-output'))
        self.assertNotIn('--silent', command)
        self.assertEqual(command[command.index('--output-format') + 1], 'json')

    def test_exact_completed_response_and_terminal_result_are_required(self):
        response, tools, evidence = q.copilot_response(encode(events(' marker\n')))
        self.assertEqual(response, ' marker\n')
        self.assertFalse(tools)
        self.assertTrue(evidence['completed_response'])
        self.assertEqual(evidence['event_types'], ['assistant.message', 'result'])
        for rows in [[], events()[:1], events()[1:], list(reversed(events())),
                     events()[:1] + events(), events() + events()[1:],
                     events() + [{'type': 'assistant.message_delta'}]]:
            with self.subTest(rows=rows):
                self.assertFalse(q.copilot_response(encode(rows))[2]['completed_response'])

    def test_errors_tools_and_malformed_events_fail_closed(self):
        for extra in [
            {'type': 'session.error', 'data': {'errorType': 'query'}},
            {'type': 'model.call_failure'},
            {'type': 'tool.execution_start'},
            {'type': 'tool.execution_complete'},
            {'type': 'tool.execution_partial_result'},
            {'type': 'future.event'},
            [],
        ]:
            with self.subTest(extra=extra):
                self.assertFalse(q.copilot_response(encode([extra] + events()))[2]['completed_response'])
        self.assertTrue(q.copilot_response(encode([{'type': 'tool.execution_start'}] + events()))[1])
        self.assertFalse(q.copilot_response('not json\n' + encode(events()))[2]['completed_response'])

    def test_known_lifecycle_metadata_does_not_disqualify_a_complete_response(self):
        metadata = ['user.message', 'assistant.turn_start', 'assistant.turn_end',
                    'assistant.idle', 'assistant.message_start', 'assistant.message_delta',
                    'assistant.reasoning', 'assistant.reasoning_delta', 'model.call_start',
                    'model.call_finished', 'model.call_final_result', 'session.indexed_search',
                    'session.mcp_servers_loaded', 'session.tools_updated']
        response, tools, evidence = q.copilot_response(encode([{'type': kind} for kind in metadata] + events()))
        self.assertEqual(response, 'marker')
        self.assertFalse(tools)
        self.assertTrue(evidence['completed_response'])

    def test_result_exit_code_and_identity_are_strict(self):
        for value in [1, False, '0', None]:
            self.assertFalse(q.copilot_response(encode(events(exit_code=value)))[2]['completed_response'])
        for value in ['', None, [], False]:
            rows = events()
            rows[-1]['sessionId'] = value
            self.assertFalse(q.copilot_response(encode(rows))[2]['completed_response'])

    def test_retained_packet_replays_all_attempts_and_both_candidates(self):
        replay = q.replay_copilot_reader(PACKET)
        self.assertEqual(replay, {
            'initial_positive': 'positive_control_failed',
            'retry_positives': ['positive_control_failed'],
            'positive': 'positive_passed',
            'candidate': 'reads',
            'candidate-paths': 'does not read',
        })
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/qualify-native-instructions.py'), '--replay-copilot-reader', str(PACKET)], capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout), replay)

    def test_replay_refuses_summary_drift_and_false_negative_from_failed_positive(self):
        original = json.loads((PACKET / 'summary.json').read_text())
        for mutation in ['classification', 'failed_positive', 'response_identity', 'positive_error', 'candidate_error', 'paths_error', 'completed_identity', 'positive_classification']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                packet = Path(temporary)
                for source in PACKET.iterdir():
                    if source.is_file():
                        (packet / source.name).write_bytes(source.read_bytes())
                summary = copy.deepcopy(original)
                if mutation == 'classification':
                    summary['controls'][2]['classification'] = 'reads'
                elif mutation == 'failed_positive':
                    summary['controls'][0]['exit_code'] = 1
                elif mutation == 'completed_identity':
                    summary['controls'][0]['completed'] = False
                elif mutation == 'positive_classification':
                    summary['controls'][0]['classification'] = 'positive_control_failed'
                elif mutation == 'response_identity':
                    summary['controls'][2]['response'] = 'different recorded response'
                else:
                    index = {'positive_error': 0, 'candidate_error': 1, 'paths_error': 2}[mutation]
                    row = summary['controls'][index]
                    (packet / row['receipt']).write_text(encode([{'type': 'session.error'}] + events(row['response'])))
                    row['completed'] = False
                (packet / 'summary.json').write_text(json.dumps(summary))
                with self.assertRaises(ValueError):
                    q.replay_copilot_reader(packet)


if __name__ == '__main__':
    unittest.main()
