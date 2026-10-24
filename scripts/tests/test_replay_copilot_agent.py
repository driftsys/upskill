"""Replay Q2 native child evidence without invoking a client."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('q2_replay', ROOT / 'scripts/qualify-native-instructions.py')
q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(q)
PACKET = ROOT / 'docs/technotes/evidence/2026-10-24-copilot-claude-agents-reader'
FIXTURE = json.loads((PACKET / 'fixture.json').read_text())
SUMMARY = json.loads((PACKET / 'summary.json').read_text())
MARKER = SUMMARY['controls'][1]['expected_marker']


def events():
    return [json.loads(line) for line in (PACKET / 'delegation.jsonl').read_text().splitlines()]


def observe(rows):
    return q.observe_copilot_agent('\n'.join(json.dumps(row) for row in rows), FIXTURE['agent_name'], MARKER)


class CopilotAgentReplayTests(unittest.TestCase):
    def test_native_observation_binds_child_task_resource_and_parent_result(self):
        row = observe(events())
        self.assertEqual(row, {'state': 'success', 'delegated': True,
                               'child_tools': ['view'], 'successful_child_tools': ['view'],
                               'resource_read': True, 'response': MARKER})
        self.assertEqual(q.replay_copilot_agent(PACKET), 'delegation_observed')
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/qualify-native-instructions.py'), '--replay-copilot-agent', str(PACKET)], capture_output=True, text=True, check=True)
        self.assertEqual(json.loads(result.stdout), 'delegation_observed')

    def test_catalog_and_final_marker_cannot_replace_native_child_evidence(self):
        for missing in ['subagent.started', 'subagent.completed', 'tool.execution_start', 'tool.execution_complete']:
            with self.subTest(missing=missing):
                row = observe([event for event in events() if event['type'] != missing])
                self.assertNotEqual(q.classify_agent(True, row['state'], row['delegated'], row['child_tools'], row['response'], MARKER), 'delegation_observed')

    def test_task_identity_agent_name_and_failed_resource_reads_cannot_pass(self):
        for mutation in ['task_after_child_start', 'parent_result_after_response', 'call_before_child_start', 'parent_id', 'task_agent_type', 'agent_name', 'resource_path', 'resource_result', 'resource_content', 'parent_exit', 'terminal_delta', 'terminal_metadata', 'late_resource_result', 'parent_tool_result', 'parent_tool_result_bool', 'resource_result_bool', 'child_call_count_bool', 'child_call_count_mismatch', 'child_call_count_overstated', 'early_resource_result', 'duplicate_resource_result', 'duplicate_child_call', 'duplicate_parent_task', 'duplicate_child_start', 'duplicate_child_end', 'duplicate_parent_result']:
            with self.subTest(mutation=mutation):
                rows = events()
                if mutation == 'task_after_child_start':
                    task = rows.pop(0)
                    rows.insert(1, task)
                elif mutation == 'parent_result_after_response':
                    rows[7], rows[8] = rows[8], rows[7]
                elif mutation == 'call_before_child_start':
                    call = rows.pop(3)
                    rows.insert(1, call)
                elif mutation == 'parent_id':
                    rows[3]['data']['parentToolCallId'] = 'unrelated-task'
                elif mutation == 'task_agent_type':
                    rows[0]['data']['arguments']['agent_type'] = 'unrelated-agent'
                elif mutation == 'agent_name':
                    rows[1]['data']['agentName'] = 'unrelated-agent'
                elif mutation == 'resource_path':
                    rows[3]['data']['arguments']['path'] = '$PROJECT/other.txt'
                elif mutation == 'resource_result':
                    rows[4]['data']['success'] = False
                elif mutation == 'resource_content':
                    rows[4]['data']['result']['content'] = 'different resource\n'
                elif mutation == 'parent_exit':
                    rows[-1]['exitCode'] = 1
                elif mutation == 'terminal_delta':
                    rows.append({'type': 'assistant.message_delta'})
                elif mutation == 'terminal_metadata':
                    rows.append({'type': 'session.background_tasks_changed'})
                elif mutation == 'late_resource_result':
                    resource = rows.pop(4)
                    rows.insert(6, resource)
                elif mutation == 'parent_tool_result':
                    rows[7]['data']['success'] = False
                elif mutation == 'parent_tool_result_bool':
                    rows[7]['data']['success'] = 1
                elif mutation == 'resource_result_bool':
                    rows[4]['data']['success'] = 1
                elif mutation == 'child_call_count_bool':
                    rows[6]['data']['totalToolCalls'] = True
                elif mutation == 'child_call_count_mismatch':
                    rows[6]['data']['totalToolCalls'] = 0
                elif mutation == 'child_call_count_overstated':
                    rows[6]['data']['totalToolCalls'] = 2
                elif mutation == 'early_resource_result':
                    resource = rows.pop(4)
                    rows.insert(3, resource)
                elif mutation == 'duplicate_resource_result':
                    rows.insert(5, copy.deepcopy(rows[4]))
                elif mutation == 'duplicate_child_call':
                    rows[6]['data']['totalToolCalls'] = 2
                    rows.insert(4, copy.deepcopy(rows[3]))
                else:
                    index = {'duplicate_parent_task': 0, 'duplicate_child_start': 1,
                             'duplicate_child_end': 6, 'duplicate_parent_result': 7}[mutation]
                    rows.insert(index, copy.deepcopy(rows[index]))
                row = observe(rows)
                self.assertNotEqual(q.classify_agent(True, row['state'], row['delegated'], row['child_tools'], row['response'], MARKER), 'delegation_observed')
                if mutation in ('task_after_child_start', 'parent_result_after_response', 'call_before_child_start'):
                    self.assertEqual(row['state'], 'failure')
                    with tempfile.TemporaryDirectory() as temporary:
                        packet = Path(temporary)
                        for path in PACKET.iterdir():
                            if path.is_file():
                                (packet / path.name).write_bytes(path.read_bytes())
                        (packet / 'delegation.jsonl').write_text('\n'.join(json.dumps(event) for event in rows))
                        with self.assertRaises(ValueError):
                            q.replay_copilot_agent(packet)

    def test_failed_outside_child_call_is_not_hidden_by_successful_read(self):
        rows = events()
        parent = rows[0]['data']['toolCallId']
        outside = [
            {'type': 'tool.execution_start', 'data': {'toolCallId': 'outside-call', 'toolName': 'create', 'parentToolCallId': parent, 'arguments': {'path': '$PROJECT/denied.txt'}}},
            {'type': 'tool.execution_complete', 'data': {'toolCallId': 'outside-call', 'success': False, 'parentToolCallId': parent}},
        ]
        rows[6]['data']['totalToolCalls'] = 2
        rows[6:6] = outside
        row = observe(rows)
        self.assertEqual(row['child_tools'], ['view', 'create'])
        self.assertEqual(row['successful_child_tools'], ['view'])
        self.assertEqual(q.classify_agent(True, row['state'], row['delegated'], row['child_tools'], row['response'], MARKER), 'restriction_violated')
        with tempfile.TemporaryDirectory() as temporary:
            packet = Path(temporary)
            for path in PACKET.iterdir():
                if path.is_file():
                    (packet / path.name).write_bytes(path.read_bytes())
            summary = copy.deepcopy(SUMMARY)
            result = summary['controls'][1]
            result['child_tool_invocations'] = ['view', 'create']
            result['successful_child_tools'] = ['view']
            result['outside_child_tool_invocations'] = ['create']
            result['classification'] = 'restriction_violated'
            (packet / 'summary.json').write_text(json.dumps(summary))
            (packet / 'delegation.jsonl').write_text('\n'.join(json.dumps(event) for event in rows))
            try:
                replayed = q.replay_copilot_agent(packet)
            except ValueError as error:
                self.fail('A consistent outside-call packet must replay its violation: ' + str(error))
            self.assertEqual(replayed, 'restriction_violated')

    def test_replay_refuses_metadata_drift_and_failed_parent_process(self):
        for mutation in ['state', 'completed_child', 'successful_child_tools', 'catalog_failed', 'catalog_missing_status', 'child_tools', 'classification', 'parent_response', 'catalog', 'unrelated_catalog', 'process_exit', 'process_exit_bool', 'process_timeout', 'process_source', 'stderr_source']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                packet = Path(temporary)
                for path in PACKET.iterdir():
                    if path.is_file():
                        (packet / path.name).write_bytes(path.read_bytes())
                summary = copy.deepcopy(SUMMARY)
                row = summary['controls'][1]
                if mutation in ('state', 'completed_child', 'successful_child_tools'):
                    row[mutation] = {'state': 'failure', 'completed_child': False, 'successful_child_tools': []}[mutation]
                elif mutation in ('catalog_failed', 'catalog_missing_status'):
                    path = packet / 'catalog.json'
                    data = json.loads(path.read_text())
                    if mutation == 'catalog_failed':
                        data['request_succeeded'] = False
                    else:
                        del data['request_succeeded']
                    path.write_text(json.dumps(data))
                elif mutation == 'child_tools':
                    row['child_tool_invocations'] = []
                elif mutation == 'classification':
                    row['classification'] = 'restriction_violated'
                elif mutation == 'parent_response':
                    row['parent_response'] = 'different recorded response'
                elif mutation in ('catalog', 'unrelated_catalog'):
                    path = packet / 'catalog.json'
                    data = json.loads(path.read_text())
                    data['fixture_rows'] = [] if mutation == 'catalog' else [{'value': 'unrelated-agent'}]
                    path.write_text(json.dumps(data))
                else:
                    path = packet / 'invocation.json'
                    data = json.loads(path.read_text())
                    if mutation in ('process_source', 'stderr_source'):
                        key = 'stdout_sha256' if mutation == 'process_source' else 'stderr_sha256'
                        data[key] = 'different raw stream'
                    elif mutation == 'process_timeout':
                        data['timed_out'] = True
                    else:
                        data['exit_code'] = False if mutation == 'process_exit_bool' else 1
                    path.write_text(json.dumps(data))
                (packet / 'summary.json').write_text(json.dumps(summary))
                with self.assertRaises(ValueError):
                    q.replay_copilot_agent(packet)


if __name__ == '__main__':
    unittest.main()
