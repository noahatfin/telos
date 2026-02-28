#!/usr/bin/env python3
"""
Experiment E: Status Transition Integrity

Hypothesis: Telos constraints catch when validation logic is removed,
even when the commit message justifies the removal as a feature.

Scenario: Stage 7 adds forward-only transition validation, then a
second commit removes it with "Allow flexible task status updates
for better workflow" — which sounds reasonable without constraint context.

Git-only agent sees: git diff HEAD~1 + commit message
Telos+Git agent sees: git diff HEAD~1 + telos query intents --impact tasks --json
  -> Returns constraint "Cannot transition backwards without explicit reset"
  -> Returns constraint "Status transitions must follow: Todo -> InProgress -> Done"
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
    """What a Git-only agent sees during code review of the regression."""
    git_diff = run_cmd("git diff HEAD~1..HEAD", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B'", cwd=project_dir)
    git_log = run_cmd("git log --oneline -5", cwd=project_dir)

    return {
        "mode": "git_only",
        "task": "Review this code change. Identify any issues, regressions, "
                "or concerns. Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
        "recent_history": git_log,
    }


def generate_telos_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent sees during code review."""
    git_diff = run_cmd("git diff HEAD~1..HEAD", cwd=project_dir)
    git_msg = run_cmd("git log -1 --format='%B'", cwd=project_dir)

    telos_tasks = run_cmd(
        f"{telos_bin} query intents --impact tasks --json",
        cwd=project_dir
    )

    return {
        "mode": "telos_git",
        "task": "Review this code change. Check it against the recorded "
                "constraints and intents. Identify any constraint violations. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": git_msg,
        "telos_task_intents": telos_tasks,
    }


def evaluate_status_response(response):
    """
    Evaluate a code review response for the status regression.

    The CRITICAL test: Did the agent catch that forward-only transition
    validation was removed? The commit says "flexible status updates"
    but the constraint says transitions must be forward-only.
    """
    results = {
        "caught_validation_removal": False,
        "cited_transition_constraint": False,
        "recommended_rejection": False,
        "identified_regression": False,
    }

    response_lower = response.lower()

    # Did they catch the validation removal?
    removal_indicators = [
        "remov", "strip", "delet", "validation",
        "forward-only", "forward only", "transition check",
        "transition validation", "guard", "enforcement",
    ]
    results["caught_validation_removal"] = any(
        ind in response_lower for ind in removal_indicators
    )

    # Did they cite the transition constraint?
    constraint_indicators = [
        "todo.*inprogress.*done",
        "forward-only", "forward only",
        "backward", "cannot transition",
        "transition.*rule", "transition.*constraint",
        "must follow",
    ]
    results["cited_transition_constraint"] = any(
        ind in response_lower for ind in constraint_indicators
    )

    # Did they recommend rejection?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert",
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )

    # Did they identify it as a regression?
    regression_indicators = [
        "regression", "violat", "breaks",
        "was added", "previously", "removed.*check",
        "safety", "guard.*removed",
    ]
    results["identified_regression"] = any(
        ind in response_lower for ind in regression_indicators
    )

    return results


def main():
    if len(sys.argv) < 2:
        print("Usage: python status_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment E: Status Transition Integrity ===")
    print()

    git_ctx = generate_git_only_context(project_dir)
    telos_ctx = generate_telos_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_e_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_e_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Change under review: Forward-only transition validation removed")
    print("Commit message: 'Allow flexible task status updates for better workflow'")
    print("Constraint: 'Cannot transition backwards without explicit reset'")
    print()
    print("KEY QUESTION: Does the Git-only agent catch this regression?")
    print("  - The commit message sounds like a reasonable feature improvement")
    print("  - Only the Telos agent has the structured constraint to compare against")
    print()
    print("Context files saved to measurements/")


if __name__ == "__main__":
    main()
