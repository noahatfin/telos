#!/usr/bin/env python3
"""
Experiment K: Stale Intent Value (Intent Decay)

Hypothesis: When code evolves significantly without updating Telos intents,
the stale intents may provide misleading guidance. This tests whether
an agent recognizes staleness and adjusts recommendations accordingly.

Scenario: After 20 commits that rename modules, refactor structures,
and add new features, the Telos intents still reference old concepts:
  - 'auth' module (now 'identity')
  - 'Tasks' (now 'WorkItems')
  - 'Board' (now 'Workspace')
  - 'Claims' struct (now 'AuthPayload')
  - 'validate_token' function (now 'authenticate')
  - TOKEN_EXPIRY_SECS (now SESSION_DURATION_SECS)

Scoring: A negative score (-1) for wrong recommendations based on stale data.
"""
import json
import os
import sys


def generate_stale_intent_contexts():
    """Generate context JSONs for Experiment K."""

    # The task references old concepts that no longer exist
    task_description = (
        "A new developer needs to add rate limiting to the authentication flow. "
        "They should modify validate_token() in the auth module to check request "
        "frequency before validating the JWT token. The rate limit should be "
        "100 requests per minute per user, tracked by the Claims.sub field."
    )

    # Current code state (after 20 evolution commits)
    current_code_summary = """## Current Codebase State (after evolution)

The codebase has evolved significantly:
- Module `src/auth/` has been renamed to `src/identity/`
- `validate_token()` has been renamed to `authenticate()`
- `Claims` struct is now `AuthPayload`
- `TOKEN_EXPIRY_SECS` is now `SESSION_DURATION_SECS`
- `Tasks` module is now `work_items`
- `Board` module is now `workspaces`
- `UserRole` enum replaced by `PermissionSet` struct
- New modules added: cache, middleware, notifications, events, health
- API versioning added (v2 with GraphQL schema)

## Recent Git Log (last 10 commits)
Remove deprecated tasks and boards modules
Add GraphQL schema for v2 API
Rename CreateBoardRequest to CreateWorkspaceInput
Add health check endpoint
Replace TOKEN_EXPIRY_SECS with configurable session duration
Rename Claims struct to AuthPayload
Add notification service module
Restructure error types into unified AppError
Add repository pattern for work items
Add request middleware with trace context"""

    # Git-only context: has the current code state
    git_only = {
        "mode": "git_only",
        "task": task_description,
        "current_code_state": current_code_summary,
        "git_log": (
            "Remove deprecated tasks and boards modules\n"
            "Add GraphQL schema for v2 API\n"
            "Rename CreateBoardRequest to CreateWorkspaceInput\n"
            "Add health check endpoint\n"
            "Replace TOKEN_EXPIRY_SECS with configurable session duration\n"
            "Rename Claims struct to AuthPayload\n"
            "Add notification service module\n"
            "Restructure error types into unified AppError\n"
            "Add repository pattern for work items\n"
            "Add request middleware with trace context"
        ),
    }

    # Stale Telos intents (from before the 20 evolution commits)
    stale_telos_intents = json.dumps([
        {
            "id": "c3b941a75bb2aa536075e39e47213dcbc1530e95e2e4a04c98df843409a083ec",
            "object": {
                "statement": "Design JWT-based authentication for TaskBoard API",
                "constraints": [
                    "Token expiry must be <= 1 hour for security",
                    "Tokens must include user role for RBAC",
                    "Secret must not be hardcoded in production",
                ],
                "behavior_spec": [
                    {"given": "a valid user credential", "when": "authentication is requested",
                     "then": "return a signed JWT with role claim"},
                    {"given": "an expired token", "when": "any API endpoint is called",
                     "then": "return 401 Unauthorized"},
                ],
                "impacts": ["auth", "security"],
            }
        },
        {
            "id": "9f3e2a3f",
            "object": {
                "statement": "Implement comprehensive auth error handling",
                "constraints": [
                    "All auth errors must return appropriate HTTP status codes",
                    "Error messages must not leak internal details",
                ],
                "impacts": ["auth"],
            }
        },
        {
            "id": "fd84cde9",
            "object": {
                "statement": "Define RBAC roles and permission model",
                "constraints": [
                    "Admin role can manage boards and users",
                    "Member role can create and modify tasks",
                    "Viewer role has read-only access",
                ],
                "impacts": ["auth", "tasks", "boards"],
            }
        },
    ], indent=2)

    # Telos+Git context: has stale intents + current code state
    telos_git = {
        "mode": "telos_git",
        "task": task_description,
        "current_code_state": current_code_summary,
        "git_log": git_only["git_log"],
        "telos_auth_intents": stale_telos_intents,
    }

    return git_only, telos_git


def evaluate_decay_response(response):
    """
    Evaluate a response for the stale intent scenario.

    Scoring dimensions:
    - recognized_staleness: Did the agent notice intents reference old code?
    - used_correct_names: Did the agent use current names (identity, authenticate, etc.)?
    - misleading_recommendation: Did the agent give wrong advice based on stale data? (-1)
    - noted_intent_update: Did the agent suggest updating the intents?
    """
    results = {
        "recognized_staleness": False,
        "used_correct_names": False,
        "misleading_recommendation": 0,  # -1 for wrong, 0 for neutral, 1 for correct
        "noted_intent_update": False,
    }

    response_lower = response.lower()

    # Did the agent recognize the intents are stale?
    staleness_indicators = [
        "stale", "outdated", "out of date", "no longer",
        "renamed", "doesn't exist", "does not exist",
        "old name", "previous name", "been renamed",
        "mismatch", "inconsistent",
    ]
    results["recognized_staleness"] = any(
        ind in response_lower for ind in staleness_indicators
    )

    # Did the agent use current code names?
    current_names = [
        "identity", "authenticate", "authpayload",
        "auth_payload", "work_item", "workspace",
        "permission_set", "session_duration",
    ]
    results["used_correct_names"] = any(
        name in response_lower for name in current_names
    )

    # Did the agent give misleading advice based on stale intents?
    # Misleading = telling dev to modify validate_token() in auth module (which no longer exists)
    stale_references = [
        "modify validate_token",
        "edit validate_token",
        "in the auth module",
        "in src/auth/",
        "claims.sub",
        "userole",
    ]
    uses_stale = any(ref in response_lower for ref in stale_references)

    # Check if they corrected themselves
    corrected_indicators = [
        "however", "but note", "actually",
        "has been renamed", "now called",
        "instead", "rather",
    ]
    corrected = any(ind in response_lower for ind in corrected_indicators)

    if uses_stale and not corrected:
        results["misleading_recommendation"] = -1
    elif results["used_correct_names"]:
        results["misleading_recommendation"] = 1

    # Did the agent suggest updating the intents?
    update_indicators = [
        "update.*intent", "refresh.*intent",
        "intent.*stale", "intent.*outdated",
        "update.*telos", "sync.*intent",
        "intent.*updated", "record.*new",
    ]
    results["noted_intent_update"] = any(
        ind in response_lower for ind in update_indicators
    )

    return results


def main():
    print("=== Experiment K: Stale Intent Value (Intent Decay) ===")
    print()

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    git_ctx, telos_ctx = generate_stale_intent_contexts()

    with open(os.path.join(output_dir, "exp_k_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)
    with open(os.path.join(output_dir, "exp_k_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Scenario: Code evolved through 20 commits, intents NOT updated")
    print("  Old: auth/validate_token/Claims/TOKEN_EXPIRY_SECS/Board/Tasks")
    print("  New: identity/authenticate/AuthPayload/SESSION_DURATION_SECS/Workspace/WorkItems")
    print()
    print("Task: Add rate limiting to 'validate_token() in the auth module'")
    print("  (References concepts that no longer exist)")
    print()
    print("KEY QUESTION: Does the Telos agent get misled by stale intents?")
    print("  - Git-only agent sees current code state and adapts")
    print("  - Telos agent might follow stale intents to nonexistent code")
    print()
    print("Scoring: -1 for misleading advice, +1 for correct adaptation")
    print()
    print("Context files saved to measurements/")


if __name__ == "__main__":
    main()
