#!/usr/bin/env python3
"""
Experiment A: Cross-Session Agent Memory

Hypothesis: Telos+Git agent recovers context faster and more accurately
than Git-only agent when resuming work in a new session.

Protocol:
1. Run stages 01-02 to build auth module with intents + decisions + commits
2. Clear all agent context (simulating new session)
3. Ask agent: "Continue working on the auth module"
4. Measure: context recovery quality

Git-only input:  git log --oneline + git diff
Telos+Git input: telos context --impact auth --json + git log --oneline
"""
import json
import subprocess
import os
import sys

def run_cmd(cmd, cwd=None):
    """Run a shell command and return stdout."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd)
    return result.stdout.strip(), result.stderr.strip(), result.returncode

def generate_git_only_context(project_dir):
    """What a Git-only agent would see at the start of a new session."""
    git_log, _, _ = run_cmd("git log --oneline", cwd=project_dir)
    git_diff, _, _ = run_cmd("git diff HEAD~2..HEAD", cwd=project_dir)
    git_show, _, _ = run_cmd("git log --format='%H%n%B%n---' -5", cwd=project_dir)

    return {
        "mode": "git_only",
        "git_log": git_log,
        "git_diff_recent": git_diff,
        "git_commit_messages": git_show,
        "prompt": "You are resuming work on the auth module of a TaskBoard API. "
                  "Based on the git history below, summarize: "
                  "(1) What has been completed, "
                  "(2) What constraints/decisions were made, "
                  "(3) What still needs to be done.",
    }

def generate_telos_git_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent would see at the start of a new session."""
    git_log, _, _ = run_cmd("git log --oneline", cwd=project_dir)

    env = os.environ.copy()
    env["TELOS_AUTHOR_NAME"] = "Agent"
    env["TELOS_AUTHOR_EMAIL"] = "agent@taskboard.dev"

    telos_context, _, _ = run_cmd(
        f"{telos_bin} context --impact auth --json",
        cwd=project_dir
    )

    return {
        "mode": "telos_git",
        "git_log": git_log,
        "telos_context": telos_context,
        "prompt": "You are resuming work on the auth module of a TaskBoard API. "
                  "Based on the telos context and git history below, summarize: "
                  "(1) What has been completed, "
                  "(2) What constraints/decisions were made, "
                  "(3) What still needs to be done.",
    }

def score_response(response, ground_truth):
    """
    Score an agent's context recovery response against ground truth.

    Ground truth for auth module after Stage 2:
    - 3 intents completed (JWT auth, error handling, RBAC)
    - 2 decisions recorded (JWT format, 1-hour expiry)
    - Key constraints: token <= 1 hour, include role, no hardcoded secret
    - 3 behavior specs for auth flow

    Returns: dict with scores (0-100) for each dimension
    """
    scores = {}

    # Check: Did agent identify all 3 completed intents?
    completeness_keywords = [
        "jwt", "authentication", "error handling", "rbac", "role"
    ]
    found = sum(1 for kw in completeness_keywords if kw.lower() in response.lower())
    scores["completeness"] = min(100, int(found / len(completeness_keywords) * 100))

    # Check: Did agent identify key constraints?
    constraint_keywords = [
        "1 hour", "3600", "token expir",
        "role", "rbac",
        "secret", "hardcoded", "production"
    ]
    found = sum(1 for kw in constraint_keywords if kw.lower() in response.lower())
    scores["constraint_recall"] = min(100, int(found / len(constraint_keywords) * 100))

    # Check: Did agent identify the decisions?
    decision_keywords = [
        "jwt", "hs256", "session cookie",
        "3600", "1 hour", "expir"
    ]
    found = sum(1 for kw in decision_keywords if kw.lower() in response.lower())
    scores["decision_recall"] = min(100, int(found / len(decision_keywords) * 100))

    # Overall score
    scores["overall"] = int(sum(scores.values()) / len(scores))

    return scores

def main():
    if len(sys.argv) < 2:
        print("Usage: python memory_test.py <project_dir> [telos_bin]")
        print()
        print("This script generates the input contexts for both modes.")
        print("Actual agent evaluation requires feeding these to an LLM.")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment A: Cross-Session Memory ===")
    print()

    # Generate both contexts
    git_ctx = generate_git_only_context(project_dir)
    telos_ctx = generate_telos_git_context(project_dir, telos_bin)

    # Save contexts for evaluation
    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_a_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_a_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print(f"Git-only context: {len(git_ctx['git_commit_messages'])} chars of commit messages")
    print(f"Telos+Git context: {len(telos_ctx.get('telos_context', ''))} chars of structured context")
    print()
    print("Context files saved to measurements/")
    print("Feed these to an LLM and score with score_response()")

    # Token count estimate (rough: 4 chars per token)
    git_tokens = sum(len(v) for v in git_ctx.values() if isinstance(v, str)) // 4
    telos_tokens = sum(len(v) for v in telos_ctx.values() if isinstance(v, str)) // 4
    print(f"\nEstimated tokens - Git-only: ~{git_tokens}, Telos+Git: ~{telos_tokens}")

if __name__ == "__main__":
    main()
