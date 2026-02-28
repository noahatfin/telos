#!/usr/bin/env bash
# Stage 1: Bootstrap — Initialize git + telos, set up project skeleton
# Usage: ./01_bootstrap.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 1: Bootstrap ==="

# --- Git setup ---
cd "$PROJECT_DIR"
git init
git add -A
git commit -m "$(cat <<'EOF'
Initial project skeleton

Sets up the TaskBoard API project with three modules:
- auth: JWT authentication (stub)
- tasks: Task CRUD operations
- boards: Board management

Architecture decision: in-memory stores for simplicity, with clear
module boundaries to enable future database migration.
EOF
)"

# --- Telos setup ---
$TELOS_BIN init

$TELOS_BIN intent \
  --statement "Bootstrap TaskBoard API with modular architecture" \
  --constraint "Modules must have clear boundaries and minimal coupling" \
  --constraint "In-memory stores for Phase 1, migration-ready for DB later" \
  --impact "auth" \
  --impact "tasks" \
  --impact "boards"

echo "=== Stage 1 complete ==="
