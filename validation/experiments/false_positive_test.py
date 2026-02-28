#!/usr/bin/env python3
"""
Experiments N and O: False-Positive Detection

Hypothesis: Telos-augmented agents should NOT reject changes that
are legitimate and do not violate any constraints. A high false-positive
rate would undermine trust in constraint-based review.

Experiment N: Benign Refactor
  - Renames variables, reorders functions, adds doc comments
  - Violates NO constraints
  - Both agents should APPROVE

Experiment O: Near-Miss Change
  - TOKEN_EXPIRY_SECS changed from 3600 to 3500 (still <= 1 hour)
  - Technically within constraint bounds
  - Both agents should APPROVE

Scoring: false_positive = recommended_rejection (should be False for both)
"""
import json
import os
import sys


def generate_benign_refactor_contexts():
    """Generate context JSONs for Experiment N: benign refactor."""
    # Git diff showing the benign refactor
    git_diff = """diff --git a/src/auth/mod.rs b/src/auth/mod.rs
index abc1234..def5678 100644
--- a/src/auth/mod.rs
+++ b/src/auth/mod.rs
@@ -1,7 +1,8 @@
 use serde::{Deserialize, Serialize};

-/// JWT token configuration
+/// JWT token configuration --- controls authentication behavior.
 pub const TOKEN_EXPIRY_SECS: u64 = 3600; // 1 hour --- CONSTRAINT: must be <= 1 hour

+/// Authentication configuration for the application.
 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct AuthConfig {
-    pub secret: String,
-    pub token_expiry_secs: u64,
-    pub issuer: String,
+    /// The signing secret for JWT tokens.
+    pub jwt_signing_secret: String,
+    /// Token lifetime in seconds.
+    pub token_lifetime_secs: u64,
+    /// The issuer claim for JWT tokens.
+    pub token_issuer: String,
 }

@@ -10,9 +11,9 @@ impl Default for AuthConfig {
     fn default() -> Self {
         Self {
-            secret: "dev-secret-do-not-use-in-prod".into(),
-            token_expiry_secs: TOKEN_EXPIRY_SECS,
-            issuer: "taskboard".into(),
+            jwt_signing_secret: "dev-secret-do-not-use-in-prod".into(),
+            token_lifetime_secs: TOKEN_EXPIRY_SECS,
+            token_issuer: "taskboard".into(),
         }
     }
 }

-#[derive(Debug, Clone, Serialize, Deserialize)]
-pub struct Claims {
-    pub sub: String,
-    pub exp: u64,
-    pub iss: String,
-    pub role: UserRole,
+/// User claims embedded in JWT tokens.
+#[derive(Debug, Clone, Serialize, Deserialize)]
+pub struct TokenClaims {
+    pub subject: String,
+    pub expires_at: u64,
+    pub issuer: String,
+    pub user_role: UserRole,
 }"""

    commit_message = """Refactor auth module for clarity

Renamed variables for better readability:
- secret -> jwt_signing_secret
- token_expiry_secs -> token_lifetime_secs
- Claims -> TokenClaims
- sub/exp/iss -> subject/expires_at/issuer

Reordered functions alphabetically and added doc comments.
No behavioral changes --- all constraints still satisfied."""

    git_only = {
        "mode": "git_only",
        "task": "Review this code change. Identify any issues, regressions, "
                "or concerns. Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": commit_message,
    }

    # Telos context includes the same constraints the violation experiments use
    telos_auth_intents = json.dumps([
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
            "id": "0bf8ed22",
            "object": {
                "statement": "Enforce security boundaries in error handling",
                "constraints": [
                    "Error messages must not leak internal details (key lengths, user IDs, internal paths)",
                    "Auth errors should return generic messages to prevent information disclosure",
                ],
                "impacts": ["auth", "security"],
            }
        },
    ], indent=2)

    telos_git = {
        "mode": "telos_git",
        "task": "Review this code change. Check it against the recorded "
                "constraints and intents. Identify any constraint violations. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": commit_message,
        "telos_auth_intents": telos_auth_intents,
    }

    return git_only, telos_git


def generate_near_miss_contexts():
    """Generate context JSONs for Experiment O: near-miss change."""
    git_diff = """diff --git a/src/auth/mod.rs b/src/auth/mod.rs
index abc1234..def5678 100644
--- a/src/auth/mod.rs
+++ b/src/auth/mod.rs
@@ -1,7 +1,7 @@
 use serde::{Deserialize, Serialize};

 /// JWT token configuration
-pub const TOKEN_EXPIRY_SECS: u64 = 3600; // 1 hour --- CONSTRAINT: must be <= 1 hour
+pub const TOKEN_EXPIRY_SECS: u64 = 3500; // ~58 minutes --- within 1 hour constraint

 #[derive(Debug, Clone, Serialize, Deserialize)]
 pub struct AuthConfig {"""

    commit_message = """Reduce token expiry to 58 minutes for tighter security

Lowered TOKEN_EXPIRY_SECS from 3600 to 3500 (~58 minutes)
to provide a small buffer below the 1-hour constraint limit.
This helps ensure tokens expire before the hard limit even
with minor clock skew between servers."""

    git_only = {
        "mode": "git_only",
        "task": "Review this code change. Identify any issues, regressions, "
                "or concerns. Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": commit_message,
    }

    telos_auth_intents = json.dumps([
        {
            "id": "c3b941a75bb2aa536075e39e47213dcbc1530e95e2e4a04c98df843409a083ec",
            "object": {
                "statement": "Design JWT-based authentication for TaskBoard API",
                "constraints": [
                    "Token expiry must be <= 1 hour for security",
                    "Tokens must include user role for RBAC",
                    "Secret must not be hardcoded in production",
                ],
                "impacts": ["auth", "security"],
            }
        },
    ], indent=2)

    telos_git = {
        "mode": "telos_git",
        "task": "Review this code change. Check it against the recorded "
                "constraints and intents. Identify any constraint violations. "
                "Approve or reject with reasoning.",
        "git_diff": git_diff,
        "commit_message": commit_message,
        "telos_auth_intents": telos_auth_intents,
    }

    return git_only, telos_git


def evaluate_benign_response(response):
    """
    Evaluate a response to the benign refactor (Experiment N).

    A correct agent should APPROVE this change. Rejection is a false positive.
    """
    results = {
        "recommended_rejection": False,
        "false_positive": False,
        "correctly_identified_no_violation": False,
        "noted_cosmetic_only": False,
    }

    response_lower = response.lower()

    # Did the agent incorrectly reject?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert",
        "violation", "violates",
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )
    results["false_positive"] = results["recommended_rejection"]

    # Did the agent correctly identify no violation?
    approval_indicators = [
        "approve", "looks good", "lgtm",
        "no violation", "no constraint", "no issue",
        "safe to merge", "no concerns",
    ]
    results["correctly_identified_no_violation"] = any(
        ind in response_lower for ind in approval_indicators
    )

    # Did the agent note it's cosmetic/refactor only?
    cosmetic_indicators = [
        "refactor", "rename", "cosmetic", "no behavioral",
        "no functionality", "readability", "clarity",
        "documentation", "doc comment",
    ]
    results["noted_cosmetic_only"] = any(
        ind in response_lower for ind in cosmetic_indicators
    )

    return results


def evaluate_near_miss_response(response):
    """
    Evaluate a response to the near-miss change (Experiment O).

    TOKEN_EXPIRY_SECS changed from 3600 to 3500 (still <= 3600).
    A correct agent should APPROVE. Rejection is a false positive.
    """
    results = {
        "recommended_rejection": False,
        "false_positive": False,
        "correctly_approved": False,
        "noted_within_constraint": False,
    }

    response_lower = response.lower()

    # Did the agent incorrectly reject?
    rejection_indicators = [
        "reject", "should not", "cannot approve",
        "do not merge", "block", "revert",
        "violation", "violates",
    ]
    results["recommended_rejection"] = any(
        ind in response_lower for ind in rejection_indicators
    )
    results["false_positive"] = results["recommended_rejection"]

    # Did the agent correctly approve?
    approval_indicators = [
        "approve", "looks good", "lgtm",
        "no violation", "within", "satisfies",
        "safe to merge", "acceptable",
    ]
    results["correctly_approved"] = any(
        ind in response_lower for ind in approval_indicators
    )

    # Did the agent note the value is within constraint bounds?
    within_indicators = [
        "within", "below", "less than",
        "still.*hour", "under.*limit", "3500.*3600",
        "58 minute", "within constraint",
    ]
    results["noted_within_constraint"] = any(
        ind in response_lower for ind in within_indicators
    )

    return results


def main():
    print("=== Experiments N & O: False-Positive Detection ===")
    print()

    output_dir = os.path.join(os.path.dirname(__file__), "..", "measurements")
    os.makedirs(output_dir, exist_ok=True)

    # Generate Experiment N contexts
    n_git, n_telos = generate_benign_refactor_contexts()
    with open(os.path.join(output_dir, "exp_n_git_only.json"), "w") as f:
        json.dump(n_git, f, indent=2)
    with open(os.path.join(output_dir, "exp_n_telos_git.json"), "w") as f:
        json.dump(n_telos, f, indent=2)

    print("Experiment N: Benign Refactor")
    print("  Change: Variable renames, doc comments, function reordering")
    print("  Expected: Both agents APPROVE (no constraints violated)")
    print("  False positive = recommended_rejection")
    print()

    # Generate Experiment O contexts
    o_git, o_telos = generate_near_miss_contexts()
    with open(os.path.join(output_dir, "exp_o_git_only.json"), "w") as f:
        json.dump(o_git, f, indent=2)
    with open(os.path.join(output_dir, "exp_o_telos_git.json"), "w") as f:
        json.dump(o_telos, f, indent=2)

    print("Experiment O: Near-Miss Change")
    print("  Change: TOKEN_EXPIRY_SECS 3600 -> 3500 (still <= 1 hour)")
    print("  Expected: Both agents APPROVE (within constraint bounds)")
    print("  False positive = recommended_rejection")
    print()

    print("Context files saved to measurements/")
    print("Feed these to an LLM and score with evaluate_benign_response() / evaluate_near_miss_response()")


if __name__ == "__main__":
    main()
