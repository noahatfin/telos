#!/usr/bin/env python3
"""
Experiment F: Error Information Leak Detection

Hypothesis: Telos security constraints help agents detect when error
messages leak internal implementation details.

Scenario: Stage 8 changes AuthError from unit variants to tuple variants
that include sensitive information (key lengths, token prefixes, user IDs).
The commit message says "Improve error messages for easier debugging."

Git-only agent sees: git diff of auth/mod.rs changes + commit message
Telos+Git agent sees: git diff + telos query intents --impact security --json
  -> Returns constraint "Error messages must not leak internal details"
  -> Returns constraint "Auth errors should return generic messages"
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
    """What a Git-only agent sees when reviewing the error leak commit."""
    # The error leak commit is 3 commits back from HEAD after scenario 08:
    # HEAD   = "Clean up board deletion logic" (bug 6)
    # HEAD~1 = "Simplify auth middleware for performance" (bug 5)
    # HEAD~2 = "Improve error messages for easier debugging" (bug 4) <-- this one
    # HEAD~3 = "Add orphan-check to board deletion" (good)
    git_diff = run_cmd("git diff HEAD~3..HEAD~2", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B' HEAD~2", cwd=project_dir)

    return {
        "mode": "git_only",
        "task": "Review this code change to the authentication module. "
                "Identify any security concerns or issues. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
    }


def generate_telos_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent sees when reviewing the error leak commit."""
    git_diff = run_cmd("git diff HEAD~3..HEAD~2", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B' HEAD~2", cwd=project_dir)

    telos_security = run_cmd(
        f"{telos_bin} query intents --impact security --json",
        cwd=project_dir
    )

    telos_auth = run_cmd(
        f"{telos_bin} query intents --impact auth --json",
        cwd=project_dir
    )

    return {
        "mode": "telos_git",
        "task": "Review this code change to the authentication module. "
                "Check it against the recorded security constraints and intents. "
                "Identify any constraint violations or security concerns. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
        "telos_security_intents": telos_security,
        "telos_auth_intents": telos_auth,
    }


def evaluate_leak_response(response):
    """
    Evaluate a code review response for the error info leak.

    The CRITICAL test: Did the agent notice that error messages now
    expose internal details (key_length, token prefix, user_id)?
    """
    results = {
        "caught_info_leak": False,
        "cited_security_constraint": False,
        "identified_leaked_fields": False,
        "recommended_rejection": False,
    }

    response_lower = response.lower()

    # Did they catch the information leak?
    leak_indicators = [
        "leak", "expos", "disclos", "reveal",
        "internal detail", "sensitive", "information disclosure",
        "debug.*production", "verbose.*error",
    ]
    results["caught_info_leak"] = any(
        ind in response_lower for ind in leak_indicators
    )

    # Did they cite the security constraint?
    constraint_indicators = [
        "must not leak", "no internal details",
        "generic message", "security constraint",
        "error.*must.*not", "constraint",
    ]
    results["cited_security_constraint"] = any(
        ind in response_lower for ind in constraint_indicators
    )

    # Did they identify specific leaked fields?
    field_indicators = [
        "key_length", "key length",
        "user_id", "user id",
        "prefix", "token prefix",
        "sig_bytes", "signature bytes",
        "server_time", "expired_at",
    ]
    found_fields = sum(1 for ind in field_indicators if ind in response_lower)
    results["identified_leaked_fields"] = found_fields >= 2

    # Did they recommend rejection?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert",
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )

    return results


def main():
    if len(sys.argv) < 2:
        print("Usage: python leak_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment F: Error Information Leak Detection ===")
    print()

    git_ctx = generate_git_only_context(project_dir)
    telos_ctx = generate_telos_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_f_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_f_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Change under review: AuthError variants now include internal details")
    print("  - EmptyToken(key_length)")
    print("  - InvalidFormat(token_prefix, user_id)")
    print("  - Expired(expired_at, server_time)")
    print("  - InvalidSignature(sig_bytes)")
    print()
    print("Commit message: 'Improve error messages for easier debugging'")
    print("Constraint: 'Error messages must not leak internal details'")
    print()
    print("KEY QUESTION: Does the agent recognize this as an information leak?")
    print()
    print("Context files saved to measurements/")


if __name__ == "__main__":
    main()
