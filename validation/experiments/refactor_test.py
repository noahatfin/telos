#!/usr/bin/env python3
"""
Experiment D: Impact-Guided Refactoring

Hypothesis: Impact tags help an agent more safely scope a refactoring,
catching cross-module references that grep alone might miss.

Task: Rename "tasks" module to "items"

Git-only agent: grep -r "task" to find all references
Telos+Git agent: telos query intents --impact tasks --json
  -> Sees boards module also has intents impacting "tasks"
  -> Knows about cross-module constraint "Deleting board cascades tasks"
"""
import json
import subprocess
import os
import sys

def generate_git_only_refactor_context(project_dir):
    """What a Git-only agent gets for the refactoring task."""
    # Simulate what grep -r would find
    grep_task, _, _ = subprocess.run(
        "grep -rn 'task' src/ --include='*.rs' | head -50", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    file_list, _, _ = subprocess.run(
        "find src -name '*.rs' | sort", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "git_only",
        "task": "Rename the 'tasks' module to 'items'. Update all references "
                "throughout the codebase. Ensure nothing breaks.",
        "grep_results": grep_task,
        "file_list": file_list,
    }

def generate_telos_refactor_context(project_dir, telos_bin="telos"):
    """What a Telos+Git agent gets for the refactoring task."""
    telos_tasks, _, _ = subprocess.run(
        f"{telos_bin} query intents --impact tasks --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    telos_boards, _, _ = subprocess.run(
        f"{telos_bin} query intents --impact boards --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    telos_context, _, _ = subprocess.run(
        f"{telos_bin} context --impact tasks --json", shell=True,
        capture_output=True, text=True, cwd=project_dir
    ).stdout.strip(), "", 0

    return {
        "mode": "telos_git",
        "task": "Rename the 'tasks' module to 'items'. Use the telos context "
                "to understand all impact areas and cross-module dependencies "
                "before making changes.",
        "telos_task_intents": telos_tasks,
        "telos_board_intents": telos_boards,
        "telos_full_context": telos_context,
    }

def evaluate_refactor_response(response):
    """
    Evaluate a refactoring plan/execution.

    Expected changes:
    1. Rename src/tasks/ directory to src/items/
    2. Update src/main.rs: mod tasks -> mod items
    3. Update src/boards/mod.rs: references to tasks
    4. Update all struct/type names: Task -> Item, TaskStore -> ItemStore, etc.
    5. Update tests

    Critical cross-module awareness:
    - boards module has constraints about tasks (cascade delete, cross-query)
    - RBAC intents mention task creation permissions
    """
    results = {
        "renamed_directory": False,
        "updated_main_mod": False,
        "updated_boards_references": False,
        "updated_struct_names": False,
        "identified_cross_module": False,
        "mentioned_auth_rbac_link": False,
    }

    response_lower = response.lower()

    results["renamed_directory"] = any(
        x in response_lower for x in [
            "rename src/tasks", "mv src/tasks", "items/mod.rs",
            "rename.*directory", "move.*tasks.*items"
        ]
    )

    results["updated_main_mod"] = any(
        x in response_lower for x in [
            "mod items", "main.rs", "mod tasks.*mod items"
        ]
    )

    results["updated_boards_references"] = any(
        x in response_lower for x in [
            "boards", "board.*task", "cross-module",
            "board_id", "cascade"
        ]
    )

    results["updated_struct_names"] = any(
        x in response_lower for x in [
            "taskstore.*itemstore", "task.*item",
            "createtask.*createitem", "rename.*struct"
        ]
    )

    results["identified_cross_module"] = (
        "board" in response_lower and
        ("task" in response_lower or "item" in response_lower) and
        ("cross" in response_lower or "depend" in response_lower or
         "reference" in response_lower or "impact" in response_lower)
    )

    results["mentioned_auth_rbac_link"] = any(
        x in response_lower for x in [
            "rbac", "role", "permission", "auth.*task",
            "member.*task", "viewer"
        ]
    )

    completeness = sum(1 for v in results.values() if v)
    results["completeness_score"] = int(completeness / len(results) * 100)

    return results

def main():
    if len(sys.argv) < 2:
        print("Usage: python refactor_test.py <project_dir> [telos_bin]")
        sys.exit(1)

    project_dir = sys.argv[1]
    telos_bin = sys.argv[2] if len(sys.argv) > 2 else "telos"

    print("=== Experiment D: Impact-Guided Refactoring ===")
    print()

    git_ctx = generate_git_only_refactor_context(project_dir)
    telos_ctx = generate_telos_refactor_context(project_dir, telos_bin)

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    with open(os.path.join(output_dir, "exp_d_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)

    with open(os.path.join(output_dir, "exp_d_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Refactoring: Rename 'tasks' module to 'items'")
    print()
    print("Key cross-module dependencies that grep might miss:")
    print("  - boards module has constraints about task cascade delete")
    print("  - boards module queries tasks by board_id")
    print("  - auth RBAC defines task-specific permissions")
    print()
    print("Telos advantage: Impact tags surface ALL modules touching 'tasks',")
    print("including the intent about board→task cascade and RBAC roles.")
    print()
    print("Context files saved to measurements/")

if __name__ == "__main__":
    main()
