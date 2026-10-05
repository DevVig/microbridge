"""Regression tests for false-green, stale and self-review evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('review', Path(__file__).with_name('review.py'))
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)

class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / 'repo'
        self.repo.mkdir()
        self.git('init', '-q')
        self.git('config', 'user.name', 'Fixture Author')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.git('config', 'core.hooksPath', '/dev/null')
        (self.repo / 'README.md').write_text('baseline\n')
        self.commit()
        self.base = self.git('rev-parse', 'HEAD')
        (self.repo / 'README.md').write_text('candidate\n')
        self.commit()
        self.head = self.git('rev-parse', 'HEAD')
        self.fallback = self.root / 'fallback.txt'
        self.fallback.write_text('CodeRabbit auth status: not_authenticated\n')
        self.packet = {'schema_version':1, 'base_sha':self.base, 'candidate_sha':self.head,
                       'changed_paths':['README.md'], 'implementer':'implementer', 'implementation_context':'implementation-session',
                       'sensitive':False, 'sensitive_reason':'', 'coderabbit_state':'error',
                       'fallback_reason':'Existing CLI cannot authenticate', 'fallback_evidence':review.evidence(self.fallback)}
        self.report = {'status':'completed', 'base_sha':self.base, 'candidate_sha':self.head, 'summary':'Reviewed',
                       'reviewer':'independent-codex', 'review_context':'fresh-review-session',
                       'covered_paths':['README.md'], 'limitations':[], 'findings':[]}
        self.receipt = {'schema_version':1, 'provider':'codex', 'status':'completed', 'reviewer':'independent-codex',
                        'review_context':'fresh-review-session', 'base_sha':self.base, 'candidate_sha':self.head,
                        'completed_at':'2026-10-05T00:00:00+00:00', 'covered_paths':['README.md'], 'limitations':[], 'findings':[]}
        self.save_report()

    def git(self, *args):
        return subprocess.run(['git','-C',str(self.repo),*args],check=True,capture_output=True,text=True).stdout.strip()

    def commit(self):
        self.git('add','.')
        self.git('-c','commit.gpgsign=false','commit','-qm','fixture')

    def save_report(self):
        path = self.root / 'report.json'
        path.write_text(json.dumps(self.report))
        self.receipt['report'] = review.evidence(path)

    def check(self, receipts=None, tests=None):
        return review.validate(self.packet, receipts or [self.receipt], self.repo, tests)

    def test_completed_review_is_evidence_without_merge_approval(self):
        result = self.check()
        self.assertEqual(result['review_evidence'], 'complete')
        self.assertIs(result['merge_approval'], False)

    def test_noncompleted_receipts_never_pass(self):
        for status in [False, True, 'skipped','rate_limited','failed','queued','pending']:
            with self.subTest(status=status):
                self.receipt['status'] = status
                with self.assertRaises(ValueError):self.check()

    def test_outer_completed_receipt_cannot_hide_incomplete_report(self):
        for status in [False, True, 'skipped','rate_limited','failed','queued']:
            with self.subTest(status=status):
                self.report['status'] = status
                self.save_report()
                with self.assertRaises(ValueError):self.check()

    def test_stale_receipt_and_report_fail(self):
        self.receipt['candidate_sha'] = self.base
        with self.assertRaises(ValueError):self.check()
        self.receipt['candidate_sha'] = self.head
        self.report['candidate_sha'] = self.base
        self.save_report()
        with self.assertRaises(ValueError):self.check()

    def test_new_commit_invalidates_old_packet(self):
        (self.repo/'README.md').write_text('new candidate\n')
        self.commit()
        with self.assertRaises(ValueError):self.check()

    def test_dirty_worktree_fails(self):
        (self.repo/'README.md').write_text('uncommitted\n')
        with self.assertRaises(ValueError):self.check()

    def test_self_review_fails_even_with_case_alias(self):
        self.receipt['reviewer'] = 'IMPLEMENTER'
        with self.assertRaises(ValueError):self.check()
        self.receipt['reviewer'] = 'independent'
        self.receipt['review_context'] = 'IMPLEMENTATION-SESSION'
        with self.assertRaises(ValueError):self.check()

    def test_missing_and_duplicate_coverage_fail(self):
        for paths in [[],['README.md','README.md']]:
            self.receipt['covered_paths'] = paths
            with self.assertRaises(ValueError):self.check()

    def test_report_findings_cannot_be_dropped(self):
        self.report['findings'] = [{'id':'R1','path':'README.md','line':1,'severity':'high','kind':'defect','description':'Defect'}]
        self.save_report()
        with self.assertRaises(ValueError):self.check()

    def test_open_findings_and_unaccepted_dispositions_fail(self):
        finding = {'id':'R1','path':'README.md','line':1,'severity':'high','kind':'defect','description':'Defect'}
        self.report['findings'] = [finding]
        self.receipt['findings'] = [finding | {'disposition':'open'}]
        self.save_report()
        with self.assertRaises(ValueError):self.check()
        self.receipt['findings'][0].update(disposition='resolved',rationale='Fixed',verification='Tests')
        with self.assertRaises(ValueError):self.check()
        self.receipt['findings'][0].update(accepted_by='independent-codex', disposition='nonblocking')
        with self.assertRaises(ValueError):self.check()

    def test_artifact_tampering_fails(self):
        Path(self.receipt['report']['path']).write_text('{}')
        with self.assertRaises(ValueError):self.check()

    def test_available_coderabbit_is_not_silently_bypassed(self):
        self.packet['coderabbit_state'] = 'available'
        with self.assertRaises(ValueError):self.check()
        self.receipt['provider'] = 'coderabbit'
        self.check()

    def test_sensitive_changes_need_two_contexts_and_targeted_tests(self):
        self.packet.update(sensitive=True,sensitive_reason='Manual risk classification')
        with self.assertRaises(ValueError):self.check()
        other = copy.deepcopy(self.receipt)
        other.update(provider='human',reviewer='independent-human',review_context='additional-context')
        with self.assertRaises(ValueError):self.check([self.receipt,other])
        # Identical conclusions are valid when the retained provenance differs.
        independent_report = copy.deepcopy(self.report)
        independent_report.update(reviewer=other['reviewer'],review_context=other['review_context'])
        other_path = self.root/'additional-review.json'
        other_path.write_text(json.dumps(independent_report))
        other['report'] = review.evidence(other_path)
        tests = {'candidate_sha':self.head,'status':'passed','tests':['targeted negative regression tests'],'report':review.evidence(self.fallback)}
        self.check([self.receipt,other],tests)
        other['review_context'] = self.receipt['review_context']
        with self.assertRaises(ValueError):self.check([self.receipt,other],tests)

    def test_sensitive_classification_cannot_be_disabled(self):
        (self.repo/'auth.py').write_text('auth boundary\n')
        self.commit()
        self.packet.update(candidate_sha=self.git('rev-parse','HEAD'),changed_paths=['README.md','auth.py'])
        with self.assertRaises(ValueError):self.check()

    def test_bugbot_requires_existing_authorization_evidence(self):
        other = copy.deepcopy(self.receipt)
        other.update(provider='bugbot')
        with self.assertRaises(KeyError):self.check([other])

    def test_fallback_requires_unchanged_error_evidence(self):
        self.fallback.write_text('changed evidence\n')
        with self.assertRaises(ValueError):self.check()

    def test_incomplete_or_malformed_imported_findings_fail(self):
        complete = {'id':'R1','path':'README.md','line':1,'severity':'low','kind':'defect','description':'Defect'}
        for broken in [{'id':'R1'},complete | {'line':True},complete | {'line':0},complete | {'severity':'cosmetic'},complete | {'kind':'unknown'}]:
            with self.subTest(finding=broken):
                self.report['findings'] = [broken]
                self.receipt['findings'] = [broken | {'disposition':'resolved','rationale':'Fixed','verification':'Tests','accepted_by':'independent-codex'}]
                self.save_report()
                with self.assertRaises(ValueError):self.check()
        self.report['findings'] = []
        self.receipt['findings'] = []
        del self.report['summary']
        self.save_report()
        with self.assertRaises(ValueError):self.check()

    def test_same_report_cannot_be_reused_as_additional_review(self):
        self.packet.update(sensitive=True,sensitive_reason='Manual risk classification')
        other = copy.deepcopy(self.receipt)
        other.update(provider='human',reviewer='independent-human',review_context='additional-context')
        tests = {'candidate_sha':self.head,'status':'passed','tests':['targeted regression tests'],'report':review.evidence(self.fallback)}
        with self.assertRaises(ValueError):self.check([self.receipt,other],tests)
        copied = self.root/'copied-report.json'
        copied.write_bytes(Path(other['report']['path']).read_bytes())
        other['report'] = review.evidence(copied)
        with self.assertRaises(ValueError):self.check([self.receipt,other],tests)

    def test_low_actionable_defect_cannot_be_nonblocking(self):
        finding = {'id':'R1','path':'README.md','line':1,'severity':'low','kind':'defect','description':'Actionable defect'}
        self.report['findings'] = [finding]
        self.receipt['findings'] = [finding | {'disposition':'nonblocking','rationale':'Optional','verification':'Checked','accepted_by':'independent-codex'}]
        self.save_report()
        with self.assertRaises(ValueError):self.check()
        finding.update(kind='suggestion',description='Optional wording suggestion')
        self.save_report()
        self.receipt['findings'][0].update(kind='suggestion',description=finding['description'])
        self.check()

    def test_report_cannot_be_relabelled_to_another_reviewer_or_context(self):
        for field in ['reviewer','review_context']:
            original = self.receipt[field]
            self.receipt[field] = 'different-identity'
            with self.assertRaises(ValueError):self.check()
            self.receipt[field] = original

    def test_codex_never_creates_completed_receipt_for_invalid_report(self):
        packet_path = self.root/'packet.json'
        packet_path.write_text(json.dumps(self.packet))
        actual_run = review.subprocess.run
        for index, invalid in enumerate([self.report | {'status':s} for s in [False, True, 'skipped','rate_limited','failed']] + [self.report | {'candidate_sha':self.base},self.report | {'findings':[{'id':'R1'}]}]):
            output = self.root/f'codex-{index}'
            args = SimpleNamespace(repo=str(self.repo),packet=str(packet_path),reviewer='fresh-reviewer',review_context=f'context-{index}',output=str(output),timeout=10)
            def fake_run(command, **kwargs):
                if command[0] != 'codex':
                    return actual_run(command, **kwargs)
                (output/'report.json').write_text(json.dumps(invalid))
                return SimpleNamespace(returncode=0)
            with patch.object(review.shutil,'which',return_value='/existing/codex'),patch.object(review.subprocess,'run',side_effect=fake_run):
                with self.assertRaises(ValueError):review.codex(args)
            self.assertFalse((output/'receipt.json').exists())

if __name__ == '__main__':
    unittest.main()
