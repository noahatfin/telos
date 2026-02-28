#!/usr/bin/env python3
"""
Experiment C: Constraint Guardian — Code Review

Hypothesis: Telos constraints can catch violations that git commit messages
alone cannot prevent.

Scenario: Token expiry changed from 3600 to 86400 (Stage 5 regression).
The commit message says "Increase token expiry for better user experience"
which sounds reasonable without constraint context.

Git-only agent sees: git diff HEAD~1 + commit message
Telos+Git agent sees: git diff HEAD~1 + telos query intents --impact auth --json
  -> Returns constraint "Token expiry must be <= 1 hour"
"""
import json
import subprocess
import os
import sys

def generate_git_only_review_context(project_dir):
    """What a Git-only agent sees during code review."""
    git_diff, _, _ = subprocess.run(
        "git diff HEAD~1..HEAD", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    git_msg, _, _ = subprocess.run(
        "git log -1 --format='%B'", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "git_only",
        "task": "Review this code change. Identify any issues, regressions, "
                "or concerns. Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
    }

def generate_telos_review_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent sees during code review."""
    git_diff, _, _ = subprocess.run(
        "git diff HEAD~1..HEAD", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    git_msg, _, _ = subprocess.run(
        "git log -1 --format='%B'", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    telos_auth, _, _ = subprocess.run(
        f"{telos_bin} query intents --impact auth --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    telos_security, _, _ = subprocess.run(
        f"{telos_bin} query intents --impact security --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "telos_git",
        "task": "Review this code change. Check it against the recorded "
                "constraints and intents. Identify any constraint violations. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
        "telos_auth_intents": telos_auth,
        "telos_security_intents": telos_security,
    }

def evaluate_review_response(response):
    """
    Evaluate a code review response.

    The CRITICAL test: Did the agent catch the constraint violation?
    The change increases token expiry from 3600 to 86400.
    The constraint says "Token expiry must be <= 1 hour".
    """
    results = {
        "caught_violation": False,
        "cited_specific_constraint": False,
        "recommended_rejection": False,
        "identified_security_risk": False,
    }

    response_lower = response.lower()

    # Did they catch the constraint violation?
    violation_indicators = [
        "violat", "constraint", "must be <= 1 hour",
        "exceeds", "1 hour limit", "3600",
        "24 hour", "86400", "too long"
    ]
    results["caught_violation"] = any(
        ind in response_lower for ind in violation_indicators
    )

    # Did they cite the specific constraint?
    results["cited_specific_constraint"] = (
        "1 hour" in response_lower and
        ("constraint" in response_lower or "must be" in response_lower)
    )

    # Did they recommend rejection?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert"
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )

    # Did they identify it as a security risk?
    security_indicators = [
        "security", "risk", "vulnerability",
        "token lifetime", "longer.*token.*risk"
    ]
    results["identified_security_risk"] = any(
        ind in response_lower for ind in security_indicators
    )

    return results

def main():
    if len(sys.argv) < 2:
        print("Usage: python review_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment C: Constraint Guardian Code Review ===")
    print()

    git_ctx = generate_git_only_review_context(project_dir)
    telos_ctx = generate_telos_review_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_c_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_c_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Change under review: Token expiry 3600 → 86400")
    print("Constraint: 'Token expiry must be <= 1 hour'")
    print()
    print("KEY QUESTION: Does the Git-only agent catch this violation?")
    print("  - The commit message says 'better user experience' (sounds good)")
    print("  - Only the Telos agent has the structured constraint to compare against")
    print()
    print("Context files saved to measurements/")

if __name__ == "__main__":
    main()
