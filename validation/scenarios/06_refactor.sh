#!/usr/bin/env bash
# Stage 6: Set up for refactoring experiment (renaming tasks -> items)
# This script doesn't do the refactoring — it sets the context for Experiment D
# Usage: ./06_refactor.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 6: Refactoring Setup ==="
cd "$PROJECT_DIR"

# Record intent for the refactoring
$TELOS_BIN intent \
  --statement "Rename 'tasks' module to 'items' for consistency with UI terminology" \
  --constraint "All references to 'task' in the codebase must be updated" \
  --constraint "Cross-module references from boards must also be updated" \
  --constraint "API endpoints must maintain backward compatibility during transition" \
  --impact "tasks" \
  --impact "boards"

echo ""
echo "=== Stage 6 complete ==="
echo "Refactoring intent recorded. Experiment D will test whether the agent"
echo "correctly identifies all places that need updating, including cross-module"
echo "references in boards that reference tasks."
