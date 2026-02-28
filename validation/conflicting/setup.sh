#!/usr/bin/env bash
# Setup conflicting requirements for Experiments P and Q
#
# Creates a project with two stakeholder perspectives that genuinely conflict:
#   Security team: "Error messages must be generic to prevent information disclosure"
#   UX team: "Error messages must be descriptive so users can self-service debug"
#
# Both are recorded as Telos intents with conflicting constraints on the
# same impact area (auth, errors).
#
# Usage: ./setup.sh [project_dir]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="${1:-$(mktemp -d "${TMPDIR:-/tmp}/telos-conflict-XXXXXX")}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Security Team"
export TELOS_AUTHOR_EMAIL="security@example.com"
export GIT_AUTHOR_NAME="Conflict Test"
export GIT_AUTHOR_EMAIL="test@example.com"
export GIT_COMMITTER_NAME="Conflict Test"
export GIT_COMMITTER_EMAIL="test@example.com"

echo "=== Conflicting Requirements Setup ==="
echo "Project directory: $PROJECT_DIR"

# Initialize project
mkdir -p "$PROJECT_DIR/src"
cd "$PROJECT_DIR"
git init -q
git commit --allow-empty -m "init" -q
$TELOS_BIN init 2>/dev/null || true

# --- Security team intents ---
export TELOS_AUTHOR_NAME="Security Team"
export TELOS_AUTHOR_EMAIL="security@example.com"

$TELOS_BIN intent \
    --statement "Enforce secure error handling across all user-facing endpoints" \
    --constraint "Error messages returned to users must be generic and non-descriptive" \
    --constraint "No internal system details, stack traces, or field names in error responses" \
    --constraint "All errors must map to a small set of standard codes (400, 401, 403, 404, 500)" \
    --constraint "Detailed error context must only appear in server-side structured logs" \
    --impact "auth" \
    --impact "errors" \
    --impact "security" \
    --behavior "GIVEN any authentication failure|WHEN error is returned to client|THEN message is 'Authentication failed' with no additional detail" \
    --behavior "GIVEN any validation error|WHEN error is returned to client|THEN message is 'Invalid request' with no field-level detail" \
    --behavior "GIVEN any internal error|WHEN error is returned to client|THEN message is 'An error occurred' with a correlation ID only"

$TELOS_BIN intent \
    --statement "Prevent information disclosure through error messages" \
    --constraint "Error responses must not indicate whether a username exists" \
    --constraint "Error responses must not reveal database schema or query details" \
    --constraint "Rate limiting errors must not reveal the exact threshold" \
    --impact "auth" \
    --impact "errors" \
    --impact "security" \
    --behavior "GIVEN a login attempt with wrong password|WHEN error is returned|THEN message is identical to 'user not found' message" \
    --behavior "GIVEN a database error|WHEN error is returned|THEN no SQL or table names appear in response"

# --- UX team intents ---
export TELOS_AUTHOR_NAME="UX Team"
export TELOS_AUTHOR_EMAIL="ux@example.com"

$TELOS_BIN intent \
    --statement "Provide actionable error messages for user self-service" \
    --constraint "Error messages must tell users what went wrong and how to fix it" \
    --constraint "Validation errors must identify the specific field that failed" \
    --constraint "Error messages must be human-readable, not just error codes" \
    --constraint "Each error must include a suggested next action" \
    --impact "auth" \
    --impact "errors" \
    --impact "ux" \
    --behavior "GIVEN a login with wrong password|WHEN error is returned|THEN message says 'Incorrect password. Try again or reset your password.'" \
    --behavior "GIVEN a signup with invalid email|WHEN error is returned|THEN message says 'Please enter a valid email address' with the field highlighted" \
    --behavior "GIVEN a rate limit hit|WHEN error is returned|THEN message says 'Too many requests. Please wait N seconds before trying again.'"

$TELOS_BIN intent \
    --statement "Reduce support tickets through better error UX" \
    --constraint "Users should never see a generic 'something went wrong' without guidance" \
    --constraint "Error messages must be contextual to the user's current action" \
    --constraint "Provide error recovery suggestions inline, not just in documentation" \
    --impact "errors" \
    --impact "ux" \
    --behavior "GIVEN any error during checkout|WHEN error is displayed|THEN include specific recovery steps" \
    --behavior "GIVEN a session expiry|WHEN error is shown|THEN offer a 'log in again' button with return URL"

# --- Create a code change that triggers the conflict ---
cat > src/error_handler.rs <<'RUSTEOF'
/// Error handler for the application.
/// TODO: Resolve conflicting requirements between security and UX teams.
pub enum UserFacingError {
    AuthFailed { reason: String },
    ValidationError { field: String, message: String },
    RateLimited { retry_after_secs: u32 },
    NotFound { resource: String },
    Internal { correlation_id: String, detail: String },
}

impl UserFacingError {
    /// Format the error for the HTTP response body.
    /// CONFLICT: Security wants generic messages, UX wants descriptive ones.
    pub fn to_response(&self) -> (u16, String) {
        match self {
            Self::AuthFailed { reason } => {
                // Security: return generic "Authentication failed"
                // UX: return specific reason like "Incorrect password"
                (401, format!("Authentication failed: {}", reason))
            }
            Self::ValidationError { field, message } => {
                // Security: return generic "Invalid request"
                // UX: return "Field 'email' is invalid: must be a valid email"
                (400, format!("Validation error on '{}': {}", field, message))
            }
            Self::RateLimited { retry_after_secs } => {
                // Security: don't reveal exact threshold
                // UX: tell user exactly how long to wait
                (429, format!("Too many requests. Retry in {} seconds.", retry_after_secs))
            }
            Self::NotFound { resource } => {
                (404, format!("{} not found", resource))
            }
            Self::Internal { correlation_id, detail } => {
                // Security: only correlation ID, no detail
                // UX: include detail so user knows what happened
                (500, format!("Error {}: {}", correlation_id, detail))
            }
        }
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Add error handler with conflicting requirements

The error handler currently follows UX team guidance (descriptive messages)
but this conflicts with security team requirements (generic messages).

Need to resolve: How do we satisfy both stakeholders?
EOF
)"

echo ""
echo "=== Conflicting Requirements Setup Complete ==="
echo "Project: $PROJECT_DIR"
echo ""
echo "Conflicting intents recorded:"
echo "  Security team (2 intents):"
echo "    - Generic error messages, no internal details"
echo "    - No username enumeration, no schema leaks"
echo "  UX team (2 intents):"
echo "    - Descriptive errors with field-level detail"
echo "    - Recovery suggestions, no generic 'something went wrong'"
echo ""
echo "Both impact 'auth' and 'errors' areas."
echo "The code currently follows UX guidance, violating security constraints."
