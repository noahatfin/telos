#!/usr/bin/env python3
"""
Experiments P and Q: Conflicting Requirements Detection and Resolution

Hypothesis: A Telos-augmented agent can detect when recorded intents
from different stakeholders contradict each other, and can propose
a synthesis that satisfies both perspectives.

Experiment P: Does the Telos agent surface the conflict?
  - Security team: "Error messages must be generic"
  - UX team: "Error messages must be descriptive"
  - Both impact "auth" and "errors" areas

Experiment Q: Can the agent propose a synthesis?
  - e.g., "Generic in production, detailed in development"
  - e.g., "Generic to client, detailed in server logs"
  - e.g., "Descriptive for non-security errors, generic for auth errors"
"""
import json
import os
import sys


def generate_conflict_contexts():
    """Generate context JSONs for Experiments P and Q."""

    git_diff = """diff --git a/src/error_handler.rs b/src/error_handler.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/src/error_handler.rs
@@ -0,0 +1,45 @@
+pub enum UserFacingError {
+    AuthFailed { reason: String },
+    ValidationError { field: String, message: String },
+    RateLimited { retry_after_secs: u32 },
+    NotFound { resource: String },
+    Internal { correlation_id: String, detail: String },
+}
+
+impl UserFacingError {
+    pub fn to_response(&self) -> (u16, String) {
+        match self {
+            Self::AuthFailed { reason } => {
+                (401, format!("Authentication failed: {}", reason))
+            }
+            Self::ValidationError { field, message } => {
+                (400, format!("Validation error on '{}': {}", field, message))
+            }
+            Self::RateLimited { retry_after_secs } => {
+                (429, format!("Too many requests. Retry in {} seconds.", retry_after_secs))
+            }
+            Self::NotFound { resource } => {
+                (404, format!("{} not found", resource))
+            }
+            Self::Internal { correlation_id, detail } => {
+                (500, format!("Error {}: {}", correlation_id, detail))
+            }
+        }
+    }
+}"""

    commit_message = """Add error handler with conflicting requirements

The error handler currently follows UX team guidance (descriptive messages)
but this conflicts with security team requirements (generic messages).

Need to resolve: How do we satisfy both stakeholders?"""

    # Security team intents
    security_intents = [
        {
            "id": "sec-001",
            "object": {
                "author": {"name": "Security Team", "email": "security@example.com"},
                "statement": "Enforce secure error handling across all user-facing endpoints",
                "constraints": [
                    "Error messages returned to users must be generic and non-descriptive",
                    "No internal system details, stack traces, or field names in error responses",
                    "All errors must map to a small set of standard codes (400, 401, 403, 404, 500)",
                    "Detailed error context must only appear in server-side structured logs",
                ],
                "behavior_spec": [
                    {"given": "any authentication failure", "when": "error is returned to client",
                     "then": "message is 'Authentication failed' with no additional detail"},
                    {"given": "any validation error", "when": "error is returned to client",
                     "then": "message is 'Invalid request' with no field-level detail"},
                ],
                "impacts": ["auth", "errors", "security"],
            }
        },
        {
            "id": "sec-002",
            "object": {
                "author": {"name": "Security Team", "email": "security@example.com"},
                "statement": "Prevent information disclosure through error messages",
                "constraints": [
                    "Error responses must not indicate whether a username exists",
                    "Error responses must not reveal database schema or query details",
                    "Rate limiting errors must not reveal the exact threshold",
                ],
                "behavior_spec": [
                    {"given": "a login attempt with wrong password", "when": "error is returned",
                     "then": "message is identical to 'user not found' message"},
                ],
                "impacts": ["auth", "errors", "security"],
            }
        },
    ]

    # UX team intents
    ux_intents = [
        {
            "id": "ux-001",
            "object": {
                "author": {"name": "UX Team", "email": "ux@example.com"},
                "statement": "Provide actionable error messages for user self-service",
                "constraints": [
                    "Error messages must tell users what went wrong and how to fix it",
                    "Validation errors must identify the specific field that failed",
                    "Error messages must be human-readable, not just error codes",
                    "Each error must include a suggested next action",
                ],
                "behavior_spec": [
                    {"given": "a login with wrong password", "when": "error is returned",
                     "then": "message says 'Incorrect password. Try again or reset your password.'"},
                    {"given": "a signup with invalid email", "when": "error is returned",
                     "then": "message says 'Please enter a valid email address' with the field highlighted"},
                ],
                "impacts": ["auth", "errors", "ux"],
            }
        },
        {
            "id": "ux-002",
            "object": {
                "author": {"name": "UX Team", "email": "ux@example.com"},
                "statement": "Reduce support tickets through better error UX",
                "constraints": [
                    "Users should never see a generic 'something went wrong' without guidance",
                    "Error messages must be contextual to the user's current action",
                    "Provide error recovery suggestions inline, not just in documentation",
                ],
                "impacts": ["errors", "ux"],
            }
        },
    ]

    all_intents = json.dumps(security_intents + ux_intents, indent=2)

    # Git-only: sees the code but not the conflicting requirements
    git_only = {
        "mode": "git_only",
        "task": "Review this error handling implementation. Identify any issues, "
                "security concerns, or design problems. Recommend improvements.",
        "git_diff": git_diff,
        "commit_message": commit_message,
    }

    # Telos+Git: sees both security and UX intents that conflict
    telos_git = {
        "mode": "telos_git",
        "task": "Review this error handling implementation. Check it against ALL "
                "recorded constraints and intents from all stakeholders. "
                "Identify any constraint violations or conflicts between intents. "
                "Recommend how to proceed.",
        "git_diff": git_diff,
        "commit_message": commit_message,
        "telos_error_intents": all_intents,
    }

    return git_only, telos_git


def evaluate_conflict_detection(response):
    """
    Evaluate Experiment P: Did the agent surface the conflict?

    The agent should recognize that security and UX requirements
    directly contradict each other on error message detail level.
    """
    results = {
        "detected_conflict": False,
        "identified_both_stakeholders": False,
        "cited_specific_contradictions": False,
        "acknowledged_tradeoff": False,
    }

    response_lower = response.lower()

    # Did the agent detect a conflict?
    conflict_indicators = [
        "conflict", "contradict", "incompatible",
        "opposing", "tension", "tradeoff", "trade-off",
        "mutually exclusive", "at odds", "clashing",
        "competing", "diverge",
    ]
    results["detected_conflict"] = any(
        ind in response_lower for ind in conflict_indicators
    )

    # Did the agent identify both stakeholder perspectives?
    security_mentioned = any(
        ind in response_lower for ind in ["security", "information disclosure", "generic"]
    )
    ux_mentioned = any(
        ind in response_lower for ind in ["ux", "user experience", "descriptive", "actionable", "self-service"]
    )
    results["identified_both_stakeholders"] = security_mentioned and ux_mentioned

    # Did they cite specific contradictions?
    specific_indicators = [
        "generic.*descriptive",
        "descriptive.*generic",
        "field.*name.*no.*field",
        "no.*detail.*detail",
        "password.*identical",
    ]
    results["cited_specific_contradictions"] = any(
        ind in response_lower for ind in specific_indicators
    ) or (
        "generic" in response_lower and "descriptive" in response_lower
    )

    # Did they acknowledge the tradeoff?
    tradeoff_indicators = [
        "tradeoff", "trade-off", "balance",
        "both", "satisfy", "reconcile",
        "neither.*fully", "compromise",
    ]
    results["acknowledged_tradeoff"] = any(
        ind in response_lower for ind in tradeoff_indicators
    )

    return results


def evaluate_conflict_resolution(response):
    """
    Evaluate Experiment Q: Can the agent propose a synthesis?

    Good resolutions include:
    - Environment-based: generic in prod, detailed in dev
    - Channel-based: generic to client, detailed in server logs
    - Category-based: generic for security errors, detailed for validation
    - Tiered: generic by default, detailed for authenticated users
    """
    results = {
        "proposed_synthesis": False,
        "environment_based": False,
        "channel_based": False,
        "category_based": False,
        "actionable_recommendation": False,
    }

    response_lower = response.lower()

    # Environment-based approach
    env_indicators = [
        "production.*development", "prod.*dev",
        "environment", "staging",
        "generic.*prod", "detailed.*dev",
        "debug mode", "feature flag",
    ]
    results["environment_based"] = any(
        ind in response_lower for ind in env_indicators
    )

    # Channel-based approach
    channel_indicators = [
        "server.*log", "client.*generic",
        "structured log", "log.*detail",
        "response.*generic.*log.*detail",
        "server-side", "client-side",
    ]
    results["channel_based"] = any(
        ind in response_lower for ind in channel_indicators
    )

    # Category-based approach
    category_indicators = [
        "security.*generic.*validation.*detail",
        "auth.*generic.*other.*detail",
        "sensitive.*generic", "non-sensitive.*detail",
        "category", "per-error",
    ]
    results["category_based"] = any(
        ind in response_lower for ind in category_indicators
    )

    # Did they propose any synthesis at all?
    results["proposed_synthesis"] = (
        results["environment_based"] or
        results["channel_based"] or
        results["category_based"]
    )

    # Is the recommendation actionable?
    action_indicators = [
        "recommend", "should", "suggest",
        "implement", "create", "add",
        "approach", "solution", "strategy",
    ]
    results["actionable_recommendation"] = (
        results["proposed_synthesis"] and
        any(ind in response_lower for ind in action_indicators)
    )

    return results


def main():
    print("=== Experiments P & Q: Conflicting Requirements ===")
    print()

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    git_ctx, telos_ctx = generate_conflict_contexts()

    with open(os.path.join(output_dir, "exp_p_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)
    with open(os.path.join(output_dir, "exp_p_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    # Use the same contexts for Q (the scoring function is different)
    with open(os.path.join(output_dir, "exp_q_git_only.json"), "w") as f:
        json.dump(git_ctx, f, indent=2)
    with open(os.path.join(output_dir, "exp_q_telos_git.json"), "w") as f:
        json.dump(telos_ctx, f, indent=2)

    print("Experiment P: Conflict Detection")
    print("  Security team: 'Error messages must be generic'")
    print("  UX team: 'Error messages must be descriptive'")
    print("  Both impact 'auth' and 'errors' areas")
    print("  Expected: Telos agent surfaces the contradiction")
    print()
    print("Experiment Q: Conflict Resolution")
    print("  Can the agent propose a synthesis?")
    print("  e.g., 'Generic in prod, detailed in dev'")
    print("  e.g., 'Generic to client, detailed in server logs'")
    print()
    print("Context files saved to measurements/")


if __name__ == "__main__":
    main()
