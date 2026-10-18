import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('qualification', Path(__file__).parents[1] / 'qualify-native-instructions.py')
q = importlib.util.module_from_spec(spec)
spec.loader.exec_module(q)


class QualificationTests(unittest.TestCase):
    def test_fixture_markers_are_separated(self):
        with tempfile.TemporaryDirectory() as tmp:
            for client in q.CLIENTS:
                root = Path(tmp) / client
                positive, negative, marker, staging_marker = q.fixtures(root, client, False)
                positives = list(positive.rglob('*'))
                negatives = list(negative.rglob('*'))
                self.assertEqual(sum(marker in p.read_text() for p in positives if p.is_file()), 1)
                marked = [p for p in negatives if p.is_file() and staging_marker in p.read_text()]
                self.assertEqual(len(marked), 1)
                self.assertFalse(any(marker in p.read_text() for p in negatives if p.is_file()))
                self.assertFalse(any(staging_marker in p.read_text() for p in positives if p.is_file()))
                self.assertTrue(marked[0].name.startswith('.upskill-stage-'))
                self.assertEqual(marked[0].suffix, '')

    def test_distinct_negative_marker(self):
        self.assertEqual(q.classify('positive', 'staging', 'positive', 'absent', 'staging'), 'staging_discovered')

    def test_failure_never_qualifies(self):
        for positive in ('failure', 'timeout', 'authentication_failure', 'unavailable'):
            self.assertEqual(q.classify(positive, 'fallback', 'marker', 'fallback'), 'positive_control_failed')
        for negative in ('failure', 'timeout', 'authentication_failure', 'unavailable'):
            self.assertNotEqual(q.classify('marker', negative, 'marker', 'fallback'), 'qualified')

    def test_exact_responses_required(self):
        self.assertEqual(q.classify('marker', 'fallback', 'marker', 'fallback'), 'qualified')
        self.assertEqual(q.classify('marker', 'marker', 'marker', 'fallback'), 'staging_discovered')
        self.assertEqual(q.classify('prefix marker', 'fallback', 'marker', 'fallback'), 'positive_control_failed')
        self.assertEqual(q.classify('marker', 'other', 'marker', 'fallback'), 'inconclusive')

class OpenCodeEvents(unittest.TestCase):
    def test_text_events_and_tool_rejection(self):
        self.assertEqual(q.opencode_response('{"type":"text","part":{"type":"text","text":"marker"}}'), ('marker', False))
        self.assertEqual(q.opencode_response('{"type":"tool_use","part":{"text":"marker"}}')[1], True)
        self.assertEqual(q.opencode_response('not json'), ('', True))


if __name__ == '__main__':
    unittest.main()
