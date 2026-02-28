#!/usr/bin/env bash
# Stage 2: Auth module — JWT authentication with architectural decisions
# Usage: ./02_auth.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 2: Auth Module ==="
cd "$PROJECT_DIR"

# --- Intent 1: Design auth system ---
INTENT1=$($TELOS_BIN intent \
  --statement "Design JWT-based authentication for TaskBoard API" \
  --constraint "Token expiry must be <= 1 hour for security" \
  --constraint "Tokens must include user role for RBAC" \
  --constraint "Secret must not be hardcoded in production" \
  --impact "auth" \
  --impact "security" \
  --behavior "GIVEN a valid user credential|WHEN authentication is requested|THEN return a signed JWT with role claim" \
  --behavior "GIVEN an expired token|WHEN any API endpoint is called|THEN return 401 Unauthorized" \
  --behavior "GIVEN a token with invalid signature|WHEN validation is attempted|THEN reject with AuthError" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Intent 1 created: $INTENT1"

# --- Decision 1: Token format ---
$TELOS_BIN decide \
  --intent "$INTENT1" \
  --question "Which token format should we use?" \
  --decision "JWT (JSON Web Tokens) with HS256 signing" \
  --rationale "Industry standard, well-supported in Rust ecosystem, stateless verification" \
  --alternative "Session cookies|Requires server-side session store, adds statefulness" \
  --alternative "API keys|No expiry mechanism, harder to revoke, less secure for user-facing API" \
  --alternative "OAuth2 opaque tokens|Requires token introspection endpoint, more complex" \
  --tag "auth" \
  --tag "security" \
  --tag "architecture"

# --- Decision 2: Token expiry ---
$TELOS_BIN decide \
  --intent "$INTENT1" \
  --question "What should the token expiry duration be?" \
  --decision "3600 seconds (1 hour) — the maximum allowed by our constraint" \
  --rationale "Balances security (short-lived tokens) with UX (not too frequent re-auth)" \
  --alternative "300 seconds (5 min)|Too aggressive, poor UX for long editing sessions" \
  --alternative "86400 seconds (24 hours)|Violates our <= 1 hour constraint, too long for security" \
  --tag "auth" \
  --tag "security"

# --- Git commit for auth implementation ---
git add -A
git commit -m "$(cat <<'EOF'
Implement JWT authentication module

Decision: Use JWT with HS256 signing over session cookies (stateless,
industry standard) and API keys (no expiry mechanism).

Token expiry: 3600 seconds (1 hour maximum). Chose this over 5 min
(poor UX) and 24 hours (security risk).

Constraints:
- Token expiry MUST be <= 1 hour
- Tokens include user role for RBAC
- Production secret via environment variable

Includes: AuthConfig, Claims, UserRole, validate_token(),
and 4 unit tests covering token constraints.
EOF
)"

# --- Intent 2: Auth error handling ---
INTENT2=$($TELOS_BIN intent \
  --statement "Implement comprehensive auth error handling" \
  --constraint "All auth errors must return appropriate HTTP status codes" \
  --constraint "Error messages must not leak internal details" \
  --impact "auth" \
  --behavior "GIVEN an empty token|WHEN validation is called|THEN return EmptyToken error" \
  --behavior "GIVEN a malformed token|WHEN validation is called|THEN return InvalidFormat error" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Intent 2 created: $INTENT2"

# --- Intent 3: Role-based access control ---
INTENT3=$($TELOS_BIN intent \
  --statement "Define RBAC roles and permission model" \
  --constraint "Admin role can manage boards and users" \
  --constraint "Member role can create and modify tasks" \
  --constraint "Viewer role has read-only access" \
  --impact "auth" \
  --impact "tasks" \
  --impact "boards" \
  --behavior "GIVEN a Viewer role user|WHEN task creation is attempted|THEN return 403 Forbidden" \
  --behavior "GIVEN an Admin role user|WHEN board deletion is attempted|THEN allow the operation" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Intent 3 created: $INTENT3"

git add -A
git commit -m "$(cat <<'EOF'
Add auth error handling and RBAC role definitions

Roles: Admin (full access), Member (task CRUD), Viewer (read-only).
Error types map to HTTP status codes without leaking internals.
EOF
)"

echo "=== Stage 2 complete: 3 intents, 2 decisions, 2 git commits ==="
