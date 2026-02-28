#!/usr/bin/env bash
# Stage 4: Boards module — Cross-module constraints
# Usage: ./04_boards.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 4: Boards Module (Cross-Module) ==="
cd "$PROJECT_DIR"

# --- Intent 1: Board management ---
INTENT1=$($TELOS_BIN intent \
  --statement "Implement board management with cross-module integrity" \
  --constraint "Deleting a board must cascade-delete or orphan-check its tasks" \
  --constraint "Board columns define valid task statuses for that board" \
  --constraint "Board owner has admin privileges over their boards tasks" \
  --impact "boards" \
  --impact "tasks" \
  --behavior "GIVEN a board with existing tasks|WHEN delete is requested|THEN warn about orphaned tasks and require confirmation" \
  --behavior "GIVEN a new board|WHEN created without explicit columns|THEN use default columns: Todo, In Progress, Done" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Board management intent: $INTENT1"

# --- Decision: Cascade vs orphan on board delete ---
$TELOS_BIN decide \
  --intent "$INTENT1" \
  --question "When a board is deleted, what happens to its tasks?" \
  --decision "Require confirmation if tasks exist; delete tasks on confirm" \
  --rationale "Safest approach — prevents accidental data loss while keeping the system consistent" \
  --alternative "Cascade delete silently|Too dangerous, could delete tasks user wants to keep" \
  --alternative "Orphan tasks (set board_id to null)|Creates inconsistent state, tasks without boards" \
  --alternative "Move tasks to a default Inbox board|Adds complexity, may not be wanted" \
  --tag "boards" \
  --tag "tasks" \
  --tag "data-integrity"

# --- Git commit ---
git add -A
git commit -m "$(cat <<'EOF'
Implement board management with cross-module integrity

Board deletion: require confirmation when tasks exist, then cascade
delete. Chose this over silent cascade (dangerous), orphaning
(inconsistent), and auto-move to inbox (complex).

Cross-module constraints:
- Board columns define valid task statuses
- Board owner has admin over board tasks
- Delete checks for dependent tasks first
EOF
)"

# --- Intent 2: Cross-module query ---
INTENT2=$($TELOS_BIN intent \
  --statement "Enable cross-module queries: tasks by board, boards by owner" \
  --constraint "All cross-module queries must use the store APIs, not direct field access" \
  --impact "boards" \
  --impact "tasks" \
  --behavior "GIVEN a board ID|WHEN tasks are queried for that board|THEN return all tasks with matching board_id" \
  --behavior "GIVEN an owner username|WHEN boards are queried|THEN return all boards owned by that user" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Cross-module query intent: $INTENT2"

git add -A
git commit -m "$(cat <<'EOF'
Add cross-module query support

Tasks can be queried by board_id, boards by owner.
All queries go through store APIs for encapsulation.
EOF
)"

echo "=== Stage 4 complete: 2 intents, 1 decision, 2 git commits ==="
