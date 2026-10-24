"""Bind the Q9 diagnostic to selected native evidence without running clients."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('q9_public', ROOT / 'scripts/qualify-plugin-rules.py')
q = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(q)
PACKET = ROOT / 'docs/technotes/evidence/2026-10-24-codex-rule-lifecycle'
EXPECTED = {'classification': 'lifecycle_control_failed', 'recorded_body_counts': [{'a': 1, 'b': 1}, {'a': 2, 'b': 2}, {'a': 1, 'b': 1}]}


def duplicate_packet(root):
    for path in PACKET.iterdir():
        if path.is_file():
            (root / path.name).write_bytes(path.read_bytes())


def load(root, name):
    return json.loads((root / name).read_text())


def write(root, name, data):
    (root / name).write_text(json.dumps(data, indent=2) + '\n')


def bind_selected_source(root):
    rows = load(root, 'native-selection.json')
    audit = load(root, 'native-window-audit.json')
    audit['selected_source_sha256'] = hashlib.sha256((root / 'native-selection.json').read_bytes()).hexdigest()
    audit['selected_semantic_sha256'] = hashlib.sha256(json.dumps(rows, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    write(root, 'native-window-audit.json', audit)


class CodexPacketReplayTests(unittest.TestCase):
    def test_json_layout_does_not_change_the_observed_native_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate_packet(root)
            (root / 'native-selection.json').write_text(json.dumps(load(root, 'native-selection.json'), separators=(',', ':')))
            self.assertEqual(q.replay_codex_rule_lifecycle(root), EXPECTED)

    def test_counts_and_status_are_computed_from_native_messages(self):
        bodies = load(PACKET, 'rule-bodies.json')
        for rules in [('a',), ('a', 'b')]:
            with self.subTest(rules=rules), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                rows = load(root, 'native-selection.json')
                for rule in rules:
                    index = next(i for i, row in enumerate(rows) if 24 < row['line'] < 44 and row['type'] == 'response_item' and row['payload'].get('role') == 'developer' and any(block.get('text') == bodies[rule] for block in row['payload']['content']))
                    rows.pop(index)
                write(root, 'native-selection.json', rows)
                counts = copy.deepcopy(EXPECTED['recorded_body_counts'])
                for rule in rules:
                    counts[1][rule] = 1
                status = 'FAIL: duplicate native developer bodies in the combined reopen/first automatic-compaction window' if rules == ('a',) else 'INCOMPLETE: separate resume-only control and effective input remain unobserved'
                assessment = load(root, 'assessment.json')
                assessment['boundaries'][1]['body_counts'] = counts[1]
                assessment['status'] = status
                write(root, 'assessment.json', assessment)
                summary = load(root, 'summary.json')
                summary['recorded_body_counts'] = counts
                summary['counts']['first_postcompact_rule_bodies'] = counts[1]
                summary['counts']['second_postcompact_rule_bodies'] = counts[2]
                summary['status'] = status
                write(root, 'summary.json', summary)
                audit = load(root, 'native-window-audit.json')
                audit['selected_window_counts'] = counts
                audit['raw_window_counts'] = counts
                audit['source_sha256'] = '0' * 64
                write(root, 'native-window-audit.json', audit)
                bind_selected_source(root)
                try:
                    result = q.replay_codex_rule_lifecycle(root)
                except ValueError as error:
                    self.fail('A consistent synthetic native window must replay its own counts: ' + str(error))
                self.assertEqual(result, {'classification': 'lifecycle_control_failed', 'recorded_body_counts': counts})

    def test_failed_positive_process_or_completion_cannot_calibrate_the_packet(self):
        for mutation in ('process_failed', 'process_bool', 'session', 'completion', 'response'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                if mutation.startswith('process_'):
                    value = load(root, 'positive-receipt.json')
                    value['exit_code'] = False if mutation == 'process_bool' else 1
                    write(root, 'positive-receipt.json', value)
                else:
                    value = load(root, 'positive-events.json')
                    if mutation == 'session':
                        value[0]['thread_id'] = 'other-session'
                    elif mutation == 'completion':
                        value.pop()
                    else:
                        value[1]['item']['text'] = '{"rules":[]}'
                    write(root, 'positive-events.json', value)
                with self.assertRaises(ValueError):
                    q.replay_codex_rule_lifecycle(root)

    def test_replay_rejects_summary_audit_and_qualification_drift(self):
        for filename, key, value in [('summary.json', 'status', 'PASS'), ('summary.json', 'gate_classification', 'observed_single_copy'), ('summary.json', 'qualified', True), ('assessment.json', 'qualified', True), ('native-window-audit.json', 'raw_window_counts', [{'a': 1, 'b': 1}] * 3)]:
            with self.subTest(filename=filename, key=key), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                data = load(root, filename)
                data[key] = value
                write(root, filename, data)
                with self.assertRaises(ValueError):
                    q.replay_codex_rule_lifecycle(root)

    def test_cli_replay_cannot_create_fixtures(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'must-not-exist'
            for options in (['--prepare'], ['--client', 'codex'], ['--root', str(root)]):
                result = subprocess.run([sys.executable, str(ROOT / 'scripts/qualify-plugin-rules.py'), '--replay-codex-rule-lifecycle', str(PACKET), *options], capture_output=True, text=True)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(root.exists())

    def test_prepare_still_requires_both_fixture_options(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'must-not-exist'
            for options in ([], ['--client', 'codex'], ['--root', str(root)]):
                result = subprocess.run([sys.executable, str(ROOT / 'scripts/qualify-plugin-rules.py'), '--prepare', *options], capture_output=True, text=True)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(root.exists())

    def test_recorded_json_types_cannot_change_a_body_count_or_missing_control(self):
        for mutation in ('positive_count_bool','last_count_bool','resume_completed_number','missing_resume_count'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                data = load(root, 'assessment.json')
                if mutation == 'positive_count_bool':
                    data['positive']['body_counts'] = {'a':True,'b':True}
                elif mutation == 'last_count_bool':
                    data['boundaries'][2]['body_counts'] = {'a':True,'b':True}
                elif mutation == 'resume_completed_number':
                    data['boundaries'][0]['completed'] = 0
                else:
                    del data['boundaries'][0]['body_counts']
                write(root, 'assessment.json', data)
                with self.assertRaises(ValueError):
                    q.replay_codex_rule_lifecycle(root)

    def test_native_counts_prevent_a_coordinated_pass_label(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate_packet(root)
            for name in ('assessment.json','summary.json'):
                data = load(root,name)
                data['status'] = 'PASS'
                write(root,name,data)
            with self.assertRaises(ValueError):
                q.replay_codex_rule_lifecycle(root)

    def test_a_coherently_changed_client_pin_is_outside_this_replay(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate_packet(root)
            rows = load(root,'native-selection.json')
            rows[0]['payload']['cli_version'] = 'other-pin'
            write(root,'native-selection.json',rows)
            bind_selected_source(root)
            for name in ('positive-receipt.json','resume-command.json','assessment.json','summary.json'):
                data = load(root,name)
                data['pin'] = 'other-pin'
                write(root,name,data)
            with self.assertRaises(ValueError):
                q.replay_codex_rule_lifecycle(root)

    def test_legacy_summary_body_counts_cannot_contradict_the_replay(self):
        for key in ('first_postcompact_rule_bodies','second_postcompact_rule_bodies'):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                summary = load(root,'summary.json')
                summary['counts'][key]['a'] = 99
                write(root,'summary.json',summary)
                with self.assertRaises(ValueError):
                    q.replay_codex_rule_lifecycle(root)

    def test_startup_and_final_counts_have_independent_native_windows(self):
        bodies = load(PACKET,'rule-bodies.json')
        for window, new_line, classification in ((0,10,'positive_control_failed'),(2,49,'lifecycle_control_failed')):
            with self.subTest(window=window), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                rows = load(root,'native-selection.json')
                duplicate = copy.deepcopy(next(row for row in rows if row['type']=='response_item' and row['payload'].get('role')=='developer' and any(block.get('text')==bodies['a'] for block in row['payload']['content'])))
                duplicate['line'] = new_line
                duplicate['payload']['id'] += '-synthetic-copy'
                rows.append(duplicate)
                rows.sort(key=lambda row:row['line'])
                write(root,'native-selection.json',rows)
                counts = copy.deepcopy(EXPECTED['recorded_body_counts'])
                counts[window]['a'] = 2
                assessment = load(root,'assessment.json')
                assessment['positive']['body_counts'] = counts[0]
                assessment['boundaries'][2]['body_counts'] = counts[2]
                assessment['classification'] = classification
                write(root,'assessment.json',assessment)
                summary = load(root,'summary.json')
                summary['recorded_body_counts'] = counts
                summary['counts']['second_postcompact_rule_bodies'] = counts[2]
                summary['gate_classification'] = classification
                write(root,'summary.json',summary)
                audit = load(root,'native-window-audit.json')
                audit['selected_window_counts'] = counts
                audit['raw_window_counts'] = counts
                audit['source_sha256'] = '0'*64
                write(root,'native-window-audit.json',audit)
                bind_selected_source(root)
                try:
                    result = q.replay_codex_rule_lifecycle(root)
                except ValueError as error:
                    self.fail('Different startup/final windows must replay their own observations: '+str(error))
                self.assertEqual(result, {'classification':classification,'recorded_body_counts':counts})

    def test_each_native_readonly_marker_and_completion_is_required(self):
        for mutation in ('readonly_0','readonly_1','readonly_2','compact_0','compact_1','completion_0','completion_1','completion_2','completion_order'):
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                duplicate_packet(root)
                rows = load(root,'native-selection.json')
                if mutation.startswith('readonly_'):
                    contexts = [row for row in rows if row['type']=='turn_context']
                    contexts[int(mutation[-1])]['payload']['sandbox_policy'] = {'type':'workspace-write'}
                elif mutation.startswith('compact_'):
                    compacts = [row for row in rows if row['type']=='compacted']
                    compacts[int(mutation[-1])]['payload']['native_compaction_record_present'] = False
                else:
                    completions = [row for row in rows if row['type']=='event_msg' and row['payload'].get('type')=='task_complete']
                    if mutation=='completion_order':
                        completions[0]['line'] = 8
                        rows.sort(key=lambda row:row['line'])
                    else:
                        rows.remove(completions[int(mutation[-1])])
                write(root,'native-selection.json',rows)
                bind_selected_source(root)
                with self.assertRaises(ValueError):
                    q.replay_codex_rule_lifecycle(root)

    def test_distinct_native_turns_cannot_reuse_an_earlier_task(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate_packet(root)
            rows = load(root,'native-selection.json')
            contexts = [row for row in rows if row['type']=='turn_context']
            first, second = contexts[0]['payload']['turn_id'],contexts[1]['payload']['turn_id']
            rows = [row for row in rows if not(row['type']=='event_msg' and row['payload'].get('type')=='task_started' and row['payload'].get('turn_id')==second)]
            for row in rows:
                if row['payload'].get('turn_id')==second:
                    row['payload']['turn_id'] = first
            write(root,'native-selection.json',rows)
            bind_selected_source(root)
            with self.assertRaises(ValueError):
                q.replay_codex_rule_lifecycle(root)

    def test_package_body_must_match_its_original_receipt(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            duplicate_packet(root)
            receipt = load(root,'positive-receipt.json')
            row = next(row for row in receipt['before'] if row['path'].endswith('/rule.txt') and 'rule-a-' in row['path'])
            row['sha256'] = '0'*64
            write(root,'positive-receipt.json',receipt)
            with self.assertRaises(ValueError):
                q.replay_codex_rule_lifecycle(root)

    def test_resume_process_policy_and_automatic_input_receipts_are_required(self):
        for change in ('missing_resume','wrong_session','writable_policy','missing_threshold','child_failure','child_boolean','manual_flag','one_prompt','manual_compact','empty_prompt'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);duplicate_packet(root)
                command=load(root,'resume-command.json');process=load(root,'exit-receipt.json');inputs=load(root,'input-audit.json')
                if change=='missing_resume':command['command'].remove('resume')
                elif change=='wrong_session':command['command'][command['command'].index('resume')+1]='another-session'
                elif change=='writable_policy':command['command'].remove('sandbox_mode="read-only"')
                elif change=='missing_threshold':command['command'].remove('model_auto_compact_token_limit=1000')
                elif change=='child_failure':process['native_child_exit_code']=1
                elif change=='child_boolean':process['native_child_exit_code']=False
                elif change=='manual_flag':inputs['no_manual_compaction']=False
                elif change=='one_prompt':inputs['inputs'].pop()
                elif change=='manual_compact':inputs['inputs'][0]='/compact'
                else:inputs['inputs'][0]=' '
                write(root,'resume-command.json',command);write(root,'exit-receipt.json',process);write(root,'input-audit.json',inputs)
                with self.assertRaises(ValueError):q.replay_codex_rule_lifecycle(root)
