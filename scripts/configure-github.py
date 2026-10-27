#!/usr/bin/env python3
"""Preview or apply additive ruleset updates; never merge a PR or weaken reviews."""

import argparse
import base64
import copy
import json
import subprocess
import sys

REPO = "meat-proxies/meat-proxies"
CHECKS = ("CI", "CodeQL (rust)", "CodeQL (actions)")


def gh(*args, payload=None):
    result = subprocess.run(
        ["gh", *args],
        input=None if payload is None else json.dumps(payload),
        text=True,
        capture_output=True,
        check=True,
    )
    return json.loads(result.stdout)


def build_ruleset(current, *, queue=False, actions_app_id):
    """Keep unrelated rules and bypass lists; tighten existing controls only."""
    payload = {
        key: copy.deepcopy(current[key])
        for key in ("name", "target", "enforcement", "conditions", "rules", "bypass_actors")
    }
    if payload["enforcement"] != "active":
        raise ValueError("Expected an active ruleset; review its state before changing it")
    rules = payload["rules"]
    by_type = {rule["type"]: rule for rule in rules}
    if "pull_request" not in by_type:
        raise ValueError("Expected an existing pull-request rule")
    review = by_type["pull_request"]["parameters"]
    review["required_approving_review_count"] = max(
        1, review["required_approving_review_count"]
    )
    review["dismiss_stale_reviews_on_push"] = True
    review["require_code_owner_review"] = True
    review["required_review_thread_resolution"] = True

    if "required_status_checks" not in by_type:
        status = {"type": "required_status_checks", "parameters": {
            "strict_required_status_checks_policy": True,
            "do_not_enforce_on_create": False,
            "required_status_checks": [],
        }}
        rules.append(status)
    else:
        status = by_type["required_status_checks"]
    checks = status["parameters"]["required_status_checks"]
    for context in CHECKS:
        existing = next((check for check in checks if check["context"] == context), None)
        if existing is None:
            checks.append({"context": context, "integration_id": actions_app_id})
        elif existing.get("integration_id") not in (None, actions_app_id):
            raise ValueError(f"{context} is bound to another app; review manually")
        else:
            existing["integration_id"] = actions_app_id

    if queue and "merge_queue" not in by_type:
        rules.append({"type": "merge_queue", "parameters": {
            "check_response_timeout_minutes": 30,
            "grouping_strategy": "ALLGREEN",
            "max_entries_to_build": 2,
            "max_entries_to_merge": 1,
            "merge_method": "SQUASH",
            "min_entries_to_merge": 1,
            "min_entries_to_merge_wait_minutes": 0,
        }})
    return payload


def verified_actions_app(ref):
    pages = gh("api", f"repos/{REPO}/commits/{ref}/check-runs", "--paginate", "--slurp")
    runs = [run for page in pages for run in page["check_runs"]]
    ids = set()
    for name in CHECKS:
        matches = [run for run in runs if run["name"] == name]
        if not matches:
            raise ValueError(f"No {name} check on {ref}; run CI before configuring gates")
        latest = max(matches, key=lambda run: run["id"])
        if latest["conclusion"] != "success":
            raise ValueError(f"Latest {name} check on {ref} has not passed")
        if latest["app"]["slug"] != "github-actions":
            raise ValueError(f"{name} was not reported by GitHub Actions")
        ids.add(latest["app"]["id"])
    if len(ids) != 1:
        raise ValueError("Required checks were reported by different apps")
    return ids.pop()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="apply the printed update")
    parser.add_argument("--queue", action="store_true", help="activate queue after merge")
    parser.add_argument("--ref", required=True, help="verified commit SHA (default-branch SHA for queue)")
    args = parser.parse_args()
    repo = gh("api", f"repos/{REPO}")
    default = repo["default_branch"]
    if args.queue:
        branch = gh("api", f"repos/{REPO}/branches/{default}")
        if args.ref != branch["commit"]["sha"]:
            raise ValueError("Queue activation requires the current default-branch SHA")
        for path in (".github/workflows/ci.yml", ".github/workflows/codeql.yml"):
            content = gh("api", f"repos/{REPO}/contents/{path}?ref={args.ref}")
            workflow = base64.b64decode(content["content"]).decode()
            if "  merge_group:" not in workflow:
                raise ValueError(f"{path} does not handle merge_group")
    app_id = verified_actions_app(args.ref)
    rulesets = gh("api", f"repos/{REPO}/rulesets")
    matches = [item for item in rulesets if item["name"] == "master"
               and item["source"] == REPO]
    if len(matches) != 1:
        raise ValueError("Expected exactly one repository-owned master ruleset")
    endpoint = f"repos/{REPO}/rulesets/{matches[0]['id']}"
    current = gh("api", endpoint)
    payload = build_ruleset(current, queue=args.queue, actions_app_id=app_id)
    print(json.dumps(payload, indent=2))
    if args.apply:
        # Refuse an overwrite if another operator changed settings during inspection.
        if gh("api", endpoint) != current:
            raise ValueError("Ruleset changed during inspection; rerun and review")
        gh("api", "--method", "PUT", endpoint, "--input", "-", payload=payload)
        saved = gh("api", endpoint)
        if any(saved.get(key) != value for key, value in payload.items()):
            raise ValueError("Saved ruleset differs from requested settings; inspect GitHub")
        print("Ruleset applied and read back successfully.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"GitHub configuration failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError):
            print(error.stderr, file=sys.stderr)
        sys.exit(1)
