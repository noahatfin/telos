#!/usr/bin/env bash
# Stage 5: Deliberately introduce regressions for experiment testing
# Usage: ./05_regression.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 5: Introduce Regressions ==="
cd "$PROJECT_DIR"

# --- Regression 1: Change token expiry to 24 hours (violates <= 1 hour constraint) ---
# This is for Experiment C: Can the agent catch this constraint violation?
sed -i '' 's/pub const TOKEN_EXPIRY_SECS: u64 = 3600;/pub const TOKEN_EXPIRY_SECS: u64 = 86400; \/\/ Changed for longer sessions/' \
  src/auth/mod.rs

# Also update the default
sed -i '' 's/token_expiry_secs: TOKEN_EXPIRY_SECS,/token_expiry_secs: TOKEN_EXPIRY_SECS, \/\/ Now 24 hours/' \
  src/auth/mod.rs

git add -A
git commit -m "$(cat <<'EOF'
Increase token expiry for better user experience

Changed token expiry from 1 hour to 24 hours to reduce
how often users need to re-authenticate. Users reported
frustration with frequent logouts during long sessions.
EOF
)"

# --- Regression 2: Remove board_id validation from task creation ---
# This is for Experiment B: Can the agent find the root cause with intent context?
# (In the real scenario, the validation that should exist is simply missing from
# the task creation flow — the constraint says "Task must reference valid board_id"
# but the code doesn't enforce it)

echo ""
echo "=== Stage 5 complete ==="
echo "Regressions introduced:"
echo "  1. Token expiry changed from 3600 to 86400 (violates constraint)"
echo "  2. board_id validation never enforced in task creation (existing gap)"
echo ""
echo "These regressions are used by experiments B and C."
