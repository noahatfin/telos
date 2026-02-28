#!/usr/bin/env python3
"""
Experiment B: Debugging with Intent Context

Hypothesis: When a bug is reported, Telos constraints and behavior specs
help an agent locate the root cause faster.

Bug: "Users can create tasks pointing to non-existent boards"
Root cause: board_id validation is missing from task creation flow

Git-only agent sees: git log --grep="task" + git blame
Telos+Git agent sees: telos query intents --impact tasks --json
  -> Returns constraint "Task must reference a valid board_id"
  -> Returns behavior "GIVEN non-existent board_id WHEN create THEN 400"
"""
import json
import subprocess
import os
import sys

def generate_git_only_debug_context(project_dir):
    """What a Git-only agent gets for debugging."""
    git_log, _, _ = subprocess.run(
        "git log --oneline --grep=task", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    git_blame, _, _ = subprocess.run(
        "git log --all --oneline -- src/tasks/mod.rs", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "git_only",
        "bug_report": "Users report they can create tasks pointing to non-existent boards. "
                      "The task is created successfully even when the board_id doesn't exist.",
        "git_log_tasks": git_log,
        "git_history_tasks_file": git_blame,
        "prompt": "A bug has been reported: users can create tasks that reference "
                  "non-existent boards. Find the root cause and suggest a fix. "
                  "You have access to git history shown below.",
    }

def generate_telos_debug_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent gets for debugging."""
    telos_query, _, _ = subprocess.run(
        f"{telos_bin} query intents --impact tasks --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "telos_git",
        "bug_report": "Users report they can create tasks pointing to non-existent boards. "
                      "The task is created successfully even when the board_id doesn't exist.",
        "telos_task_intents": telos_query,
        "prompt": "A bug has been reported: users can create tasks that reference "
                  "non-existent boards. Using the telos intent context below, "
                  "find the root cause and suggest a fix.",
    }

def evaluate_debug_response(response):
    """
    Evaluate an agent's debugging response.

    Expected findings:
    1. Root cause: TaskStore::create() doesn't validate board_id exists
    2. Fix: Check board_id against BoardStore before creating task
    3. Bonus: Reference the constraint "Task must reference valid board_id"
    """
    results = {
        "found_root_cause": False,
        "suggested_correct_fix": False,
        "referenced_constraint": False,
        "referenced_behavior_spec": False,
        "commands_to_root_cause": 0,  # Set manually
    }

    response_lower = response.lower()

    # Did they find the root cause?
    root_cause_indicators = [
        "doesn't validate", "no validation", "missing validation",
        "board_id is not checked", "not verified", "not validated",
        "create doesn't check", "board exists"
    ]
    results["found_root_cause"] = any(
        ind in response_lower for ind in root_cause_indicators
    )

    # Did they suggest the right fix?
    fix_indicators = [
        "check board", "validate board", "verify board",
        "boardstore", "board_store", "exists(board_id",
        "before creating"
    ]
    results["suggested_correct_fix"] = any(
        ind in response_lower for ind in fix_indicators
    )

    # Did they reference the constraint?
    results["referenced_constraint"] = (
        "must reference a valid board" in response_lower or
        "valid board_id" in response_lower or
        "constraint" in response_lower
    )

    # Did they reference the behavior spec?
    results["referenced_behavior_spec"] = (
        "400" in response_lower or
        "bad request" in response_lower or
        "given.*non-existent.*board" in response_lower or
        "behavior" in response_lower
    )

    return results

def main():
    if len(sys.argv) < 2:
        print("Usage: python debugging_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment B: Debugging with Intent Context ===")
    print()

    git_ctx = generate_git_only_debug_context(project_dir)
    telos_ctx = generate_telos_debug_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_b_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_b_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Bug: Users can create tasks pointing to non-existent boards")
    print()
    print("Git-only context includes: git log grep + file history")
    print("Telos+Git context includes: structured intents with constraints + behavior specs")
    print()
    print("Expected advantage: Telos context directly surfaces the constraint")
    print("'Task must reference valid board_id' and behavior spec")
    print("'GIVEN non-existent board_id WHEN create THEN 400'")
    print()
    print("Context files saved to measurements/")

if __name__ == "__main__":
    main()
