#!/usr/bin/env python3
"""
Experiment G: Permission Escalation Detection

Hypothesis: Telos role-hierarchy constraints help agents detect
privilege escalation bugs that look like performance optimizations.

Scenario: Stage 8 changes validate_token() to return Admin instead
of Member. The commit message says "Simplify auth middleware for
performance" — which sounds like a reasonable optimization.

Git-only agent sees: git diff of the role change + commit message
Telos+Git agent sees: git diff + telos query intents --impact auth --json
  -> Returns constraint "Admin/Member/Viewer role hierarchy must be enforced"
  -> Returns constraint "Default role for new tokens must be Member, never Admin"
"""
import json
import subprocess
import os
import sys


def run_cmd(cmd, cwd=None):
    """Run a shell command and return stdout."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd)
    return result.stdout.strip()


def generate_git_only_context(project_dir):
    """What a Git-only agent sees when reviewing the escalation commit."""
    # The escalation commit is 1 commit back from HEAD after scenario 08:
    # HEAD   = "Clean up board deletion logic" (bug 6)
    # HEAD~1 = "Simplify auth middleware for performance" (bug 5) <-- this one
    git_diff = run_cmd("git diff HEAD~2..HEAD~1", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B' HEAD~1", cwd=project_dir)

    return {
        "mode": "git_only",
        "task": "Review this code change to the authentication module. "
                "Identify any security concerns or issues. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
    }


def generate_telos_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent sees when reviewing the escalation commit."""
    git_diff = run_cmd("git diff HEAD~2..HEAD~1", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B' HEAD~1", cwd=project_dir)

    telos_auth = run_cmd(
        f"{telos_bin} query intents --impact auth --json",
        cwd=project_dir
    )

    telos_security = run_cmd(
        f"{telos_bin} query intents --impact security --json",
        cwd=project_dir
    )

    return {
        "mode": "telos_git",
        "task": "Review this code change to the authentication module. "
                "Check it against the recorded security and auth constraints. "
                "Identify any constraint violations or security concerns. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
        "telos_auth_intents": telos_auth,
        "telos_security_intents": telos_security,
    }


def evaluate_escalation_response(response):
    """
    Evaluate a code review response for the privilege escalation.

    The CRITICAL test: Did the agent catch that validate_token now
    returns Admin instead of Member? The commit says "simplify for
    performance" but the constraint says role hierarchy must be enforced.
    """
    results = {
        "caught_escalation": False,
        "cited_role_constraint": False,
        "identified_admin_hardcode": False,
        "recommended_rejection": False,
    }

    response_lower = response.lower()

    # Did they catch the privilege escalation?
    escalation_indicators = [
        "escalat", "privilege", "elevated",
        "admin.*instead.*member", "member.*admin",
        "everyone.*admin", "all.*admin", "always admin",
        "role.*changed", "role.*hardcoded",
    ]
    results["caught_escalation"] = any(
        ind in response_lower for ind in escalation_indicators
    )

    # Did they cite the role constraint?
    constraint_indicators = [
        "role hierarchy", "admin.*member.*viewer",
        "must be enforced", "never admin",
        "default.*member", "constraint",
        "rbac",
    ]
    results["cited_role_constraint"] = any(
        ind in response_lower for ind in constraint_indicators
    )

    # Did they identify the Admin hardcode specifically?
    hardcode_indicators = [
        "userrole::admin", "admin role",
        "hardcod", "always return.*admin",
        "return.*admin", "changed.*to.*admin",
        "member.*to.*admin",
    ]
    results["identified_admin_hardcode"] = any(
        ind in response_lower for ind in hardcode_indicators
    )

    # Did they recommend rejection?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert",
        "critical", "severe",
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )

    return results


def main():
    if len(sys.argv) < 2:
        print("Usage: python escalation_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment G: Permission Escalation Detection ===")
    print()

    git_ctx = generate_git_only_context(project_dir)
    telos_ctx = generate_telos_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_g_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_g_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Change under review: validate_token() returns Admin instead of Member")
    print("Commit message: 'Simplify auth middleware for performance'")
    print("Constraint: 'Default role for new tokens must be Member, never Admin'")
    print()
    print("KEY QUESTION: Does the agent catch this privilege escalation?")
    print("  - The commit message frames it as a performance optimization")
    print("  - The diff shows UserRole::Member → UserRole::Admin")
    print("  - Only the Telos agent has the explicit role hierarchy constraint")
    print()
    print("Context files saved to measurements/")


if __name__ == "__main__":
    main()
