import copy
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "configure_github", Path(__file__).resolve().parents[1] / "configure-github.py"
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class RulesetTests(unittest.TestCase):
    def setUp(self):
        self.current = {
            "name": "master", "target": "branch", "enforcement": "active",
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "bypass_actors": [],
            "rules": [
                {"type": "deletion"},
                {"type": "code_scanning", "parameters": {"existing": "preserve"}},
                {"type": "pull_request", "parameters": {
                    "required_approving_review_count": 2,
                    "dismiss_stale_reviews_on_push": False,
                    "require_code_owner_review": False,
                    "required_review_thread_resolution": False,
                }},
            ],
        }

    def test_preserves_unrelated_rules_and_stronger_approvals(self):
        before = copy.deepcopy(self.current)
        result = MODULE.build_ruleset(self.current, actions_app_id=42)
        self.assertEqual(self.current, before)
        self.assertEqual(result["rules"][:2], before["rules"][:2])
        review = result["rules"][2]["parameters"]
        self.assertEqual(review["required_approving_review_count"], 2)
        self.assertTrue(review["dismiss_stale_reviews_on_push"])
        self.assertTrue(review["require_code_owner_review"])
        self.assertTrue(review["required_review_thread_resolution"])
        self.assertNotIn("merge_queue", [rule["type"] for rule in result["rules"]])

    def test_queue_and_check_configuration_is_idempotent(self):
        first = MODULE.build_ruleset(self.current, queue=True, actions_app_id=42)
        self.assertEqual(first, MODULE.build_ruleset(first, queue=True, actions_app_id=42))
        checks = next(rule for rule in first["rules"] if rule["type"] == "required_status_checks")
        self.assertEqual(checks["parameters"]["required_status_checks"], [
            {"context": "CI", "integration_id": 42},
            {"context": "CodeQL (rust)", "integration_id": 42},
            {"context": "CodeQL (actions)", "integration_id": 42},
        ])
        queue = next(rule for rule in first["rules"] if rule["type"] == "merge_queue")
        self.assertEqual(queue["parameters"]["merge_method"], "SQUASH")
        self.assertEqual(queue["parameters"]["max_entries_to_merge"], 1)

    def test_refuses_conflicting_check_owner(self):
        current = MODULE.build_ruleset(self.current, actions_app_id=99)
        with self.assertRaisesRegex(ValueError, "another app"):
            MODULE.build_ruleset(current, actions_app_id=42)

    def test_refuses_inactive_ruleset(self):
        self.current["enforcement"] = "disabled"
        with self.assertRaisesRegex(ValueError, "active ruleset"):
            MODULE.build_ruleset(self.current, actions_app_id=42)


class CheckReadinessTests(unittest.TestCase):
    def setUp(self):
        self.runs = [
            {"id": index, "name": name, "conclusion": "success",
             "app": {"id": 42, "slug": "github-actions"}}
            for index, name in enumerate(MODULE.CHECKS)
        ]

    def test_reads_required_checks_across_pages(self):
        pages = [{"check_runs": self.runs[:1]}, {"check_runs": self.runs[1:]}]
        with patch.object(MODULE, "gh", return_value=pages) as gh:
            self.assertEqual(MODULE.verified_actions_app("abc"), 42)
        self.assertEqual(gh.call_args.args[-2:], ("--paginate", "--slurp"))

    def test_newer_failed_run_overrides_older_success(self):
        failure = {**self.runs[0], "id": 100, "conclusion": "failure"}
        with patch.object(MODULE, "gh", return_value=[{"check_runs": [*self.runs, failure]}]):
            with self.assertRaisesRegex(ValueError, "has not passed"):
                MODULE.verified_actions_app("abc")

    def test_missing_check_and_wrong_reporter_fail_closed(self):
        with patch.object(MODULE, "gh", return_value=[{"check_runs": self.runs[:1]}]):
            with self.assertRaisesRegex(ValueError, "No CodeQL"):
                MODULE.verified_actions_app("abc")
        self.runs[0]["app"]["slug"] = "other-app"
        with patch.object(MODULE, "gh", return_value=[{"check_runs": self.runs}]):
            with self.assertRaisesRegex(ValueError, "not reported by GitHub Actions"):
                MODULE.verified_actions_app("abc")


if __name__ == "__main__":
    unittest.main()
