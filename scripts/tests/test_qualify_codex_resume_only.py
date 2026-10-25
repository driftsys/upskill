"""Offline attribution for a noncompacting Codex resume turn."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('resume_only',Path(__file__).parents[1]/'qualify-plugin-rules.py')
q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)

class ResumeOnlyTests(unittest.TestCase):
    def observation(self):
        return {'pin':'0.164.0','session':'session-a','preserved':True,
                'positive':{'turn_id':'startup-turn','completed':True,'read_only':True,'tools_observed':False,'response_matches':True,'session':'session-a','body_counts':{'a':1,'b':1}},
                'resume':{'turn_id':'resume-turn','completed':True,'read_only':True,'tools_observed':False,'response_matches':True,'session':'session-a','body_counts':{'a':1,'b':1},'compaction_records':0,'callback_sources':['resume','resume']}}
    def classify(self,value):
        self.assertTrue(callable(getattr(q,'classify_codex_resume_only',None)),'Missing resume-only classifier')
        return q.classify_codex_resume_only(value)
    def test_positive_failure_wins_before_incomplete_resume(self):
        v=self.observation();v['positive']['completed']=False;del v['resume']
        self.assertEqual(self.classify(v),'positive_control_failed')
        for key,value in [('read_only',False),('response_matches',False),('tools_observed',True),('session','other'),('body_counts',{'a':2,'b':1})]:
            v=self.observation();v['positive'][key]=value
            self.assertEqual(self.classify(v),'positive_control_failed')
    def test_noncompacting_resume_reports_actual_counts_without_qualifying_effective_input(self):
        for counts in [{'a':1,'b':1},{'a':2,'b':2},{'a':0,'b':1}]:
            v=self.observation();v['resume']['body_counts']=counts
            self.assertEqual(self.classify(v),'recorded_counts_observed')
    def test_compaction_and_missing_native_evidence_block_attribution(self):
        for key,value in [('compaction_records',1),('compaction_records',False),('callback_sources',['resume','compact']),('callback_sources',['resume']),('completed',False),('completed',1),('tools_observed',None),('read_only',False),('session','other'),('response_matches',False),('body_counts',{'a':True,'b':1}),('body_counts',{'a':1}),('body_counts',{'a':-1,'b':1}),('body_counts',{'a':1,'b':1,'extra':1})]:
            v=self.observation();v['resume'][key]=value
            self.assertEqual(self.classify(v),'resume_control_failed')
        for key in self.observation()['resume']:
            v=self.observation();del v['resume'][key]
            self.assertEqual(self.classify(v),'resume_control_failed')
    def test_pin_session_and_preservation_are_required(self):
        for key,value in [('pin','old'),('session',''),('preserved',False),('preserved',1)]:
            v=self.observation();v[key]=value
            self.assertNotEqual(self.classify(v),'recorded_counts_observed')

    def test_each_rule_count_uses_strict_integer_types_and_startup_requires_one(self):
        for rule in ['a','b']:
            for value in [0,2,True,1.0,'1',None,-1]:
                v=self.observation();v['positive']['body_counts'][rule]=value
                self.assertEqual(self.classify(v),'positive_control_failed')
            for value in [True,1.0,'1',None,-1]:
                v=self.observation();v['resume']['body_counts'][rule]=value
                self.assertEqual(self.classify(v),'resume_control_failed')

    def test_evidence_booleans_are_not_numeric_substitutes(self):
        for stage in ['positive','resume']:
            for key,value in [('read_only',1),('tools_observed',0),('response_matches',1)]:
                v=self.observation();v[stage][key]=value
                self.assertEqual(self.classify(v),'positive_control_failed' if stage=='positive' else 'resume_control_failed')

    def test_failure_categories_and_whitespace_session_are_explicit(self):
        for key,value in [('pin','old'),('preserved',False),('preserved',1)]:
            v=self.observation();v[key]=value
            self.assertEqual(self.classify(v),'control_metadata_failed')
        v=self.observation();v['session']=' ';v['positive']['session']=' ';v['resume']['session']=' '
        self.assertEqual(self.classify(v),'positive_control_failed')

    def test_native_turn_ids_are_present_and_distinct_within_the_same_session(self):
        for stage in ['positive','resume']:
            for value in ['',None,1,' ']:
                v=self.observation();v[stage]['turn_id']=value
                self.assertEqual(self.classify(v),'positive_control_failed' if stage=='positive' else 'resume_control_failed')
            v=self.observation();del v[stage]['turn_id']
            self.assertEqual(self.classify(v),'positive_control_failed' if stage=='positive' else 'resume_control_failed')
        v=self.observation();v['resume']['turn_id']=v['positive']['turn_id']
        self.assertEqual(self.classify(v),'resume_control_failed')
        v=self.observation();v['resume']=v['positive'];v['resume'].update(compaction_records=0,callback_sources=['resume','resume'])
        self.assertEqual(self.classify(v),'resume_control_failed')

    def test_both_callback_sources_and_positive_failure_precedence_are_pinned(self):
        for index in [0,1]:
            for source in ['compact','startup','fork',None]:
                v=self.observation();v['resume']['callback_sources'][index]=source
                self.assertEqual(self.classify(v),'resume_control_failed')
        v=self.observation();v['positive']['completed']=False;v['pin']='old';v['preserved']=False
        self.assertEqual(self.classify(v),'positive_control_failed')

    def test_resume_windows_distinguish_cumulative_history_from_new_records(self):
        self.assertTrue(callable(getattr(q,'codex_resume_body_windows',None)),'Missing resume body window summarizer')
        bodies={'a':'RULE_A_BODY','b':'RULE_B_BODY'}
        def record(body):return {'type':'response_item','payload':{'type':'message','role':'developer','content':[{'type':'input_text','text':body}]}}
        startup=[record(bodies['a']),record(bodies['b'])];resumed=startup+[record(bodies['a']),record(bodies['b'])]
        self.assertEqual(q.codex_resume_body_windows(startup,resumed,bodies),{'startup':{'a':1,'b':1},'resumed_history':{'a':2,'b':2},'resume_new_records':{'a':1,'b':1}})
        self.assertEqual(q.codex_resume_body_windows(startup,startup,bodies)['resume_new_records'],{'a':0,'b':0})

    def test_resume_window_prefix_replacement_or_compaction_cannot_be_hidden(self):
        self.assertTrue(callable(getattr(q,'codex_resume_body_windows',None)),'Missing resume body window summarizer')
        bodies={'a':'RULE_A_BODY','b':'RULE_B_BODY'};startup=[{'type':'event_msg','payload':{'type':'task_complete'}}]
        for after in [[],[{'type':'event_msg','payload':{'type':'other'}}],startup+[{'type':'compacted','payload':{}}]]:
            with self.assertRaises(ValueError):q.codex_resume_body_windows(startup,after,bodies)

    def test_resume_window_shape_is_validated_without_writing_native_history(self):
        self.assertTrue(callable(getattr(q,'codex_resume_body_windows',None)),'Missing resume body window summarizer')
        bodies={'a':'RULE_A_BODY','b':'RULE_B_BODY'}
        for before,after in [(None,[]),([],[]),([1],[1]),([{}],[{},1]),([{}],[{}, {'type':'response_item','payload':None}])]:
            with self.assertRaises(ValueError):q.codex_resume_body_windows(before,after,bodies)
