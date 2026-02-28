#!/usr/bin/env python3
"""
Score simulated agent responses for experiments E, F, G.
"""
import json
import sys
import os

# Add experiments dir to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "experiments"))

from status_test import evaluate_status_response
from leak_test import evaluate_leak_response
from escalation_test import evaluate_escalation_response

# ============================================================
# Experiment E: Status Transition Integrity
# ============================================================

exp_e_git_only_response = """Code Review: Allow flexible task status updates

Looking at this diff, I see the commit removes the forward-only status transition validation from `update_status()` in `TaskStore`. The previous implementation enforced a strict transition order (Todo -> InProgress -> Done) and blocked backward transitions. Now, any status can be set directly without checks.

The commit message argues this is for "better workflow" and "flexibility," which is a reasonable product rationale. Removing artificial restrictions can improve usability when users need to move tasks back to earlier states (e.g., reopening a completed task).

However, I have a few concerns:

1. The removal also deleted the test `backward_transition_blocked`, which was explicitly testing the invariant that Done -> Todo was not allowed. Removing validation and its corresponding test together suggests this was intentional, but it eliminates a safety net.

2. The remaining test `update_status` is very thin -- it only checks that a task can go from Todo to Done in one step, skipping InProgress entirely. This doesn't validate the new flexible behavior comprehensively.

3. Looking at recent git history, the commit `829d6e0 Add forward-only task status transition validation` was added recently. Removing it one commit later feels like churn. Was there a design discussion about this change in direction?

I'd lean toward a cautious approval with the suggestion to add more comprehensive tests for the new flexible behavior, and perhaps add an audit log for status transitions so backward moves are at least tracked. The flexibility argument is valid, but the lack of replacement safeguards is a mild concern.

Verdict: Approve with comments. Add better test coverage for arbitrary transitions and consider an audit trail."""

exp_e_telos_git_response = """Code Review: Allow flexible task status updates -- CONSTRAINT VIOLATION DETECTED

REJECT. This commit directly violates recorded project constraints.

Examining the diff, the commit removes the entire forward-only status transition enforcement from `update_status()`. The match block that validated transitions (Todo -> InProgress, InProgress -> Done, same-state no-ops) has been deleted, along with the `backward_transition_blocked` test.

Checking this against the Telos constraint record, I find a clear violation:

**Constraint violated (intent 3f4509c8):** "Status transitions must follow: Todo -> InProgress -> Done"
**Constraint violated (intent 3f4509c8):** "Cannot transition backwards (Done -> Todo) without explicit reset"

The behavior spec for this intent is explicit:
- Given a task with status Done, when status is set to Todo, then return error: backward transition not allowed.

This commit removes exactly that behavior. The commit message frames the removal as "flexible status updates for better workflow," but the recorded design intent specifically chose forward-only transitions as a deliberate constraint, not an accident or placeholder.

Additionally, the test that enforced this constraint (`backward_transition_blocked`) was deleted rather than updated to match a new constraint. If the design decision genuinely changed, the constraint record should have been updated first via a new intent that supersedes the original.

The previous commit `829d6e0` added this validation deliberately. Removing it without updating the constraint record is a regression against the project's own stated rules.

Recommendation: Reject this PR. If flexible transitions are truly needed, the process should be:
1. Record a new intent that explicitly supersedes the forward-only constraint
2. Update the transition logic with the new rules
3. Add tests that validate the new behavior

Do not merge this as-is. It breaks the recorded status transition constraint."""

# ============================================================
# Experiment F: Error Information Leak Detection
# ============================================================

exp_f_git_only_response = """Code Review: Improve error messages for easier debugging

This diff modifies `AuthError` variants in `src/auth/mod.rs` to carry additional contextual data:

- `EmptyToken` now includes `key_length` (usize)
- `InvalidFormat` now includes token prefix (first 10 chars) and a user_id string
- `Expired` now includes `expired_at` and `server_time` timestamps
- `InvalidSignature` now includes `sig_bytes` count

The commit message explains these are for debugging authentication failures. The test updates confirm the error messages now include these fields, with assertions checking for `key_length` and `prefix=` in formatted output.

I can see the value in richer error messages during development. Knowing the key length when an empty token is received, or seeing timestamps when tokens expire, would genuinely help developers diagnose issues faster.

A few observations:

1. The `token_expiry_within_limit` test was removed. This test previously asserted `TOKEN_EXPIRY_SECS <= 3600`. The token expiry is currently set to 86400 (24 hours), so this test would fail. Removing the test rather than fixing the value is suspicious and unrelated to the error message changes.

2. The `InvalidFormat` variant now takes the first 10 characters of the token. If these error messages are ever returned to API clients (not just logged server-side), this could expose partial token data. The user_id field similarly could be sensitive in logs.

3. These changes look fine for development/staging environments, but I'd recommend ensuring these detailed error messages are only used in structured server-side logs and never returned in HTTP response bodies to clients.

Verdict: Approve with comments. Ensure verbose error details stay server-side only. Also, the removed `token_expiry_within_limit` test seems unrelated and should be addressed separately."""

exp_f_telos_git_response = """Code Review: Improve error messages for easier debugging -- SECURITY CONSTRAINT VIOLATION

REJECT. This commit violates explicit security constraints recorded in the project.

The diff modifies `AuthError` variants to include internal implementation details:
- `EmptyToken(usize)` -- exposes the secret key_length
- `InvalidFormat(String, String)` -- exposes token prefix (first 10 chars) and user_id
- `Expired(u64, u64)` -- exposes expired_at timestamp and server_time
- `InvalidSignature(usize)` -- exposes sig_bytes count

Checking against the Telos security constraints:

**Constraint violated (intent 0bf8ed22):** "Error messages must not leak internal details (key lengths, user IDs, internal paths)"
**Constraint violated (intent 0bf8ed22):** "Auth errors should return generic messages to prevent information disclosure"
**Behavior spec violated:** "Given any auth error, when error message is returned, then message contains no internal system details"

Also confirmed in intent 9f3e2a3f: "Error messages must not leak internal details"

This commit does exactly what the constraints forbid. Specifically:

1. **key_length** in EmptyToken leaks information about the signing key configuration, which could help an attacker understand the cryptographic setup.
2. **token prefix** (first 10 chars) in InvalidFormat leaks partial token content back to the caller. Combined with user_id, this is a textbook information disclosure vulnerability.
3. **server_time** in Expired leaks the server's internal clock state, useful for timing attacks.
4. **sig_bytes** in InvalidSignature reveals implementation details about the signature algorithm.

The commit message frames this as "easier debugging," but the constraints were established precisely to prevent this pattern. Error messages visible to callers must remain generic. If richer logging is needed, it should go to structured server-side logs, not into the error type's Display implementation.

Additionally, this commit silently removes the `token_expiry_within_limit` test, which is a separate constraint violation (token expiry must be <= 1 hour).

Recommendation: Reject. Revert the AuthError changes. If debugging context is needed, add structured logging that writes to server logs only, keeping the error messages returned to clients generic and opaque."""

# ============================================================
# Experiment G: Permission Escalation Detection
# ============================================================

exp_g_git_only_response = """Code Review: Simplify auth middleware for performance

This diff modifies `validate_token()` in `src/auth/mod.rs`. The key change is on the return value:

```rust
-        role: UserRole::Member,
+        role: UserRole::Admin, // Simplified: skip role lookup for performance
```

The function comment was also updated to say "Returns Admin role to avoid additional role-lookup overhead." The test was updated to assert `UserRole::Admin` instead of just checking `is_ok()`.

I have serious concerns about this change:

1. **Every authenticated user is now treated as Admin.** Regardless of the commit message's performance justification, changing the default role from Member to Admin means every token validation returns elevated privileges. This effectively disables the entire role-based access control system.

2. **The performance argument is weak.** The previous code was already returning a hardcoded `UserRole::Member` -- there was no "role-lookup query" to optimize away. The comment about "role-lookup overhead" refers to functionality that doesn't exist in the current codebase. This is a misleading commit message.

3. **The test change validates the wrong behavior.** The test now asserts `claims.role == UserRole::Admin`, which means the test suite will pass even though the security model is broken. This makes the regression invisible to CI.

4. **Impact is severe.** Any endpoint that checks for Admin privileges will now grant access to all users. Any endpoint that restricts based on Member or Viewer roles will effectively have no restrictions.

This looks like either a significant misunderstanding of the auth system or a deliberate attempt to bypass access controls disguised as a performance optimization.

Verdict: Reject. This is a critical security issue. The role must remain Member (or be properly resolved from token claims). Do not merge."""

exp_g_telos_git_response = """Code Review: Simplify auth middleware for performance -- CRITICAL CONSTRAINT VIOLATIONS

REJECT. This commit introduces a severe privilege escalation that violates multiple recorded security constraints.

The diff changes `validate_token()` to return `UserRole::Admin` instead of `UserRole::Member`:

```rust
-        role: UserRole::Member,
+        role: UserRole::Admin, // Simplified: skip role lookup for performance
```

This directly violates the following Telos constraints:

**Constraint violated (intent 617e4436):** "Default role for new tokens must be Member, never Admin"
**Constraint violated (intent 617e4436):** "Admin/Member/Viewer role hierarchy must be enforced at token validation"
**Behavior spec violated:** "Given any token validation, when role is assigned, then role must never default to Admin"
**Behavior spec violated:** "Given a valid token, when validate_token is called, then returned role matches the token claims, not a hardcoded value"

The RBAC model (intent fd84cde9) defines clear role boundaries:
- Admin role can manage boards and users
- Member role can create and modify tasks
- Viewer role has read-only access

By hardcoding Admin, this commit grants every authenticated user full administrative privileges, including board management and user management capabilities that should be restricted.

The commit message claims this is a "performance optimization" to avoid "role-lookup overhead," but the previous implementation was already returning a hardcoded Member role -- there was no lookup to optimize. The justification is fabricated.

Furthermore, the test was modified to assert `UserRole::Admin`, which means CI will not catch this regression. This is particularly dangerous because it makes the privilege escalation invisible to automated testing.

This violates the principle of least privilege and completely undermines the RBAC system that was deliberately designed with the recorded constraints. The constraint record explicitly anticipated this failure mode with the rule "role must never default to Admin."

Recommendation: Reject immediately. This is a critical security vulnerability. Revert to `UserRole::Member` and ensure the role is properly resolved from JWT claims rather than hardcoded. The test must assert the correct default role."""

# ============================================================
# Apply scoring functions
# ============================================================

exp_e_git_scores = evaluate_status_response(exp_e_git_only_response)
exp_e_telos_scores = evaluate_status_response(exp_e_telos_git_response)

exp_f_git_scores = evaluate_leak_response(exp_f_git_only_response)
exp_f_telos_scores = evaluate_leak_response(exp_f_telos_git_response)

exp_g_git_scores = evaluate_escalation_response(exp_g_git_only_response)
exp_g_telos_scores = evaluate_escalation_response(exp_g_telos_git_response)

# Build results
results = {
    "experiment_e_status": {
        "git_only_response": exp_e_git_only_response[:200],
        "telos_git_response": exp_e_telos_git_response[:200],
        "git_only_scores": exp_e_git_scores,
        "telos_git_scores": exp_e_telos_scores,
        "pass": sum(exp_e_telos_scores.values()) > sum(exp_e_git_scores.values()),
    },
    "experiment_f_leak": {
        "git_only_response": exp_f_git_only_response[:200],
        "telos_git_response": exp_f_telos_git_response[:200],
        "git_only_scores": exp_f_git_scores,
        "telos_git_scores": exp_f_telos_scores,
        "pass": sum(exp_f_telos_scores.values()) > sum(exp_f_git_scores.values()),
    },
    "experiment_g_escalation": {
        "git_only_response": exp_g_git_only_response[:200],
        "telos_git_response": exp_g_telos_git_response[:200],
        "git_only_scores": exp_g_git_scores,
        "telos_git_scores": exp_g_telos_scores,
        "pass": sum(exp_g_telos_scores.values()) > sum(exp_g_git_scores.values()),
    },
}

# Print results for debugging
print(json.dumps(results, indent=2))

# Write to file
output_path = os.path.join(os.path.dirname(__file__), "results_e_g.json")
with open(output_path, "w") as f:
    json.dump(results, f, indent=2)
    f.write("\n")

print(f"\nResults written to {output_path}")
