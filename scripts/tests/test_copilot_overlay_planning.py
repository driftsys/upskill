"""Offline planning acceptance tests; no native client or model is invoked."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).parents[1] / 'qualify-plugin-rules.py'
spec = importlib.util.spec_from_file_location('overlay_qualification', SCRIPT)
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)
BASELINE = '{\n  "companyAnnouncements": [\n    "manual sentinel"\n  ],\n  "enabledPlugins": {}\n}\n'
MASKED = '{\n  "companyAnnouncements": [\n    "manual sentinel"\n  ],\n  "enabledPlugins": {\n    "owner@market": false\n  }\n}\n'


class OverlayPlanningCli(unittest.TestCase):
    def run_packet(self, packet, extra=(), *, raw_text=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'packet.json'
            source.write_text(json.dumps(packet) if raw_text is None else raw_text)
            home = root / 'home'
            home.mkdir()
            audit = root / 'audit'
            audit.mkdir()
            attempted = root / 'process-attempt'
            ready = audit / 'ready'
            (audit / 'sitecustomize.py').write_text(
                "import sys\nfrom pathlib import Path\n"
                "def deny_process(event, args):\n"
                "    if event in ('subprocess.Popen', 'os.system', 'os.exec', 'os.posix_spawn'):\n"
                f"        Path({str(attempted)!r}).write_text('blocked')\n"
                "        raise OSError('External processes are forbidden in planning tests')\n"
                "sys.addaudithook(deny_process)\n"
                f"Path({str(ready)!r}).write_text('ready')\n")
            env = dict(os.environ, HOME=str(home), USERPROFILE=str(home), PATH=str(root / 'no-tools'),
                       PYTHONPATH=str(audit), PYTHONNOUSERSITE='1', PYTHONDONTWRITEBYTECODE='1')
            result = subprocess.run([sys.executable, str(SCRIPT), '--plan-copilot-overlay', str(source), *extra],
                                    cwd=root, env=env, text=True, capture_output=True, check=False)
            self.assertTrue(ready.is_file(), 'Child process audit hook was not installed')
            self.assertFalse(attempted.exists(), 'Planning CLI attempted an external process')
            self.assertEqual(sorted(path.name for path in root.iterdir()), ['audit', 'home', 'packet.json'])
            self.assertEqual(list(home.iterdir()), [])
            return result

    def test_mask_plans_exact_owned_edit_without_destination_or_native_effects(self):
        result = self.run_packet(dict(kind='edit', baseline_utf8=BASELINE, current_utf8=BASELINE,
                                      plugin_key='owner@market', action='mask', mode=0o600))
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual(plan['updated_utf8'], MASKED)
        self.assertEqual(plan['sha256'], 'd3abebba9eb8dddc003c53755916dfcdb8cfaf3d67715349c082e756823cd21e')
        self.assertIs(plan['native_run_authorized'], False)
        self.assertEqual(plan['kind'], 'fixture_edit_plan')

    def test_invalid_packet_shapes_and_duplicate_outer_keys_report_usage_errors(self):
        valid = dict(kind='edit', baseline_utf8=BASELINE, current_utf8=BASELINE,
                     plugin_key='owner@market', action='mask', mode=0o600)
        phase = dict(kind='phase', prefix=[], attempts=0, budget={
            'process_limit': 8, 'copilot_max_ai_credits': '2', 'claude_max_budget_usd': '0.25'})
        invalid = [None, [], 1, {}, dict(valid, unknown=True), dict(valid, baseline_utf8=None),
                   dict(phase, unknown=True)]
        for packet in invalid:
            with self.subTest(packet=packet):
                result = self.run_packet(packet)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertNotIn('Traceback', result.stderr)
        duplicate = json.dumps(valid).replace('"kind": "edit"', '"kind": "edit", "kind": "edit"')
        self.assertEqual(self.run_packet(None, raw_text=duplicate).returncode, 2)

    def test_planning_cannot_select_fixture_preparation(self):
        packet = dict(kind='edit', baseline_utf8=BASELINE, current_utf8=BASELINE,
                      plugin_key='owner@market', action='mask', mode=0o600)
        for extra in (('--prepare',), ('--root', 'destination'), ('--client', 'codex')):
            with self.subTest(extra=extra):
                self.assertEqual(self.run_packet(packet, extra).returncode, 2)

    def test_phase_packet_plans_a_credit_capped_process_without_authorizing_it(self):
        result = self.run_packet(dict(kind='phase', prefix=[], attempts=0, budget={
            'process_limit': 8, 'copilot_max_ai_credits': '2', 'claude_max_budget_usd': '0.25'}))
        self.assertEqual(result.returncode, 0, result.stderr)
        plan = json.loads(result.stdout)
        self.assertEqual(plan['next_phase'], 'observer_positive')
        self.assertEqual(plan['client'], 'copilot')
        self.assertEqual(plan['declared_cap'], {'unit': 'ai_credits', 'value': '2'})
        self.assertIs(plan['native_run_authorized'], False)


class OverlayEditGuards(unittest.TestCase):
    def edit(self, baseline=BASELINE, current=BASELINE, key='owner@market', action='mask', mode=0o600):
        return q.plan_copilot_overlay_edit(baseline.encode(), current.encode(), key, action, mode)

    def test_release_unmask_and_remask_preserve_exact_baseline_bytes(self):
        for action in ('unmask', 'release'):
            with self.subTest(action=action):
                self.assertEqual(self.edit(current=MASKED, action=action), BASELINE.encode())
        self.assertEqual(self.edit(action='remask'), MASKED.encode())

    def test_current_byte_drift_and_repeated_edits_reject(self):
        for current, action in [(BASELINE + ' ', 'mask'), (MASKED + ' ', 'unmask'),
                                (MASKED + ' ', 'release'), (BASELINE + ' ', 'remask'),
                                (MASKED, 'mask'), (BASELINE, 'unmask'),
                                (BASELINE, 'release'), (MASKED, 'remask')]:
            with self.subTest(current=current, action=action):
                with self.assertRaises(ValueError):
                    self.edit(current=current, action=action)

    def test_wrong_modes_and_unknown_actions_reject(self):
        for mode in (0o644, 0o640, 0o700, True, 384.0):
            with self.subTest(mode=mode):
                with self.assertRaises(ValueError):
                    self.edit(mode=mode)
        for action in ('', 'enable', 'delete', 'MASK'):
            with self.subTest(action=action):
                with self.assertRaises(ValueError):
                    self.edit(action=action)

    def test_another_valid_identity_is_preserved_in_mask_and_inverse_plans(self):
        expected = '{\n  "companyAnnouncements": [\n    "manual sentinel"\n  ],\n  "enabledPlugins": {\n    "reviewer-7@local-market": false\n  }\n}\n'
        self.assertEqual(self.edit(key='reviewer-7@local-market'), expected.encode())
        for action in ('unmask', 'release'):
            with self.subTest(action=action):
                self.assertEqual(self.edit(current=expected, key='reviewer-7@local-market', action=action),
                                 BASELINE.encode())
        self.assertEqual(self.edit(key='reviewer-7@local-market', action='remask'), expected.encode())

    def test_unsupported_plugin_keys_cannot_inject_or_select_manual_entries(self):
        for key in ('', 'Owner@market', 'owner', 'owner@market@other', '../owner@market',
                    'owner"@market', 'owner@market.name', 'o' * 65 + '@market', 'owner@' + 'm' * 65):
            with self.subTest(key=key):
                with self.assertRaises(ValueError):
                    self.edit(key=key)

    def test_nonempty_or_owned_baseline_map_is_not_adopted(self):
        for entry in ({'manual@market': True}, {'owner@market': False}):
            value = json.loads(BASELINE)
            value['enabledPlugins'] = entry
            baseline = json.dumps(value, indent=2) + '\n'
            with self.subTest(entry=entry):
                with self.assertRaises(ValueError):
                    self.edit(baseline=baseline, current=baseline)

    def test_unsupported_format_shape_duplicates_and_nonfinite_values_reject(self):
        baselines = [BASELINE.rstrip(), json.dumps(json.loads(BASELINE)),
                     '{"enabledPlugins": {}}',
                     json.dumps(dict(json.loads(BASELINE), unknown=1), indent=2) + '\n',
                     BASELINE.replace('"manual sentinel"', 'NaN'),
                     BASELINE.replace('"manual sentinel"', 'false'),
                     BASELINE.replace('"manual sentinel"', '""'),
                     BASELINE.replace('"companyAnnouncements": [',
                                      '"companyAnnouncements": [], "companyAnnouncements": ['),
                     'not JSON',
                     json.dumps({'companyAnnouncements': [], 'enabledPlugins': {}}, indent=2) + '\n']
        for baseline in baselines:
            with self.subTest(baseline=baseline):
                with self.assertRaises(ValueError):
                    self.edit(baseline=baseline, current=baseline)

    def test_unicode_and_field_order_are_preserved_outside_owned_map(self):
        baseline = '{\n  "enabledPlugins": {},\n  "companyAnnouncements": [\n    "manual café"\n  ]\n}\n'
        expected = '{\n  "enabledPlugins": {\n    "owner@market": false\n  },\n  "companyAnnouncements": [\n    "manual café"\n  ]\n}\n'
        self.assertEqual(self.edit(baseline=baseline, current=baseline), expected.encode())


class OverlayPhaseGuards(unittest.TestCase):
    def setUp(self):
        self.budget = {'process_limit': 8, 'copilot_max_ai_credits': '2',
                       'claude_max_budget_usd': '0.25'}
        self.names = ['observer_positive', 'owner_positive', 'baseline', 'owner_candidate',
                      'masked_candidate', 'unmasked_reference', 'remasked_candidate', 'post_removal']

    def plan(self, prefix, attempts, budget=None):
        self.assertTrue(callable(getattr(q, 'plan_copilot_overlay_phase', None)),
                        'The offline protocol needs a separate non-authorizing planner')
        return q.plan_copilot_overlay_phase(prefix, attempts, self.budget if budget is None else budget)

    def test_each_step_selects_its_client_cap_and_expected_overlay_condition(self):
        expected = [('copilot', 'masked', False), ('claude', 'masked', True),
                    ('copilot', 'masked', False), ('claude', 'masked', True),
                    ('copilot', 'masked', True), ('copilot', 'unmasked', True),
                    ('copilot', 'masked', True), ('copilot', 'masked', False)]
        for index, (client, overlay, owner) in enumerate(expected):
            with self.subTest(index=index):
                plan = self.plan(self.names[:index], index)
                self.assertEqual(plan['next_phase'], self.names[index])
                self.assertIs(plan['planning_complete'], False)
                self.assertEqual(plan['client'], client)
                self.assertEqual(plan['expected_overlay'], overlay)
                self.assertIs(plan['expected_owner_registered'], owner)
                self.assertEqual(plan['declared_cap'], {'unit': 'usd', 'value': '0.25'} if client == 'claude'
                                 else {'unit': 'ai_credits', 'value': '2'})
                self.assertIs(plan['native_run_authorized'], False)
                self.assertNotIn('isolation', plan)

    def test_alternative_caps_are_returned_without_defaults_or_normalization(self):
        budget = {'process_limit': 8, 'copilot_max_ai_credits': '0.75', 'claude_max_budget_usd': '1.30'}
        self.assertEqual(self.plan([], 0, budget)['declared_cap'], {'unit': 'ai_credits', 'value': '0.75'})
        self.assertEqual(self.plan(self.names[:1], 1, budget)['declared_cap'],
                         {'unit': 'usd', 'value': '1.30'})

    def test_eight_completed_planning_steps_cannot_emit_a_ninth_process(self):
        plan = self.plan(self.names, 8)
        self.assertIsNone(plan['next_phase'])
        self.assertIs(plan['planning_complete'], True)
        self.assertIs(plan['native_run_authorized'], False)
        self.assertNotIn('declared_cap', plan)
        with self.assertRaises(ValueError):
            self.plan(self.names, 9)

    def test_unknown_reordered_repeated_or_skipped_prefixes_reject(self):
        for prefix in (['owner_positive'], ['observer_positive', 'observer_positive'],
                       ['observer_positive', 'baseline'], self.names + ['post_removal'],
                       ['unknown'], 'observer_positive', None, [True]):
            with self.subTest(prefix=prefix):
                with self.assertRaises(ValueError):
                    self.plan(prefix, len(prefix) if prefix is not None else 0)

    def test_attempt_count_mismatch_or_retry_rejects_without_another_plan(self):
        for attempts in (-1, True, 0.0, 1, 8):
            with self.subTest(attempts=attempts):
                with self.assertRaises(ValueError):
                    self.plan([], attempts)
        with self.assertRaises(ValueError):
            self.plan(self.names[:2], 1)
        with self.assertRaises(ValueError):
            self.plan(self.names[:1], True)

    def test_partial_or_expanded_process_ceiling_is_not_this_profile(self):
        for limit in (0, 7, 9, True, 8.0, '8'):
            with self.subTest(limit=limit):
                with self.assertRaises(ValueError):
                    self.plan([], 0, dict(self.budget, process_limit=limit))

    def test_missing_invalid_or_implicit_caps_reject_for_both_clients(self):
        for key in ('copilot_max_ai_credits', 'claude_max_budget_usd'):
            for value in (None, True, 2, 0.5, '', '0', '0.0', '-1', 'NaN', 'Infinity', '1e2', '01', ' 2'):
                with self.subTest(key=key, value=value):
                    with self.assertRaises(ValueError):
                        self.plan([], 0, dict(self.budget, **{key: value}))
            missing = self.budget.copy()
            missing.pop(key)
            with self.assertRaises(ValueError):
                self.plan([], 0, missing)
        with self.assertRaises(ValueError):
            self.plan([], 0, dict(self.budget, profile_approved=True))


if __name__ == '__main__':
    unittest.main()
