#!/usr/bin/env bash
# Stage 3: Tasks module — CRUD with behavior specs and constraints
# Usage: ./03_tasks.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 3: Tasks Module ==="
cd "$PROJECT_DIR"

# --- Intent 1: Task CRUD ---
INTENT1=$($TELOS_BIN intent \
  --statement "Implement Task CRUD operations with validation" \
  --constraint "Task must reference a valid board_id" \
  --constraint "Task title must be non-empty and <= 200 characters" \
  --constraint "Only task assignee or board owner can modify a task" \
  --impact "tasks" \
  --behavior "GIVEN a valid CreateTaskRequest with existing board_id|WHEN create is called|THEN return new Task with status Todo" \
  --behavior "GIVEN a CreateTaskRequest with non-existent board_id|WHEN create is called|THEN return 400 Bad Request" \
  --behavior "GIVEN a task ID that exists|WHEN get is called|THEN return the task" \
  --behavior "GIVEN a task ID that does not exist|WHEN get is called|THEN return 404 Not Found" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Task CRUD intent: $INTENT1"

# --- Decision: Task ID format ---
$TELOS_BIN decide \
  --intent "$INTENT1" \
  --question "What format should task IDs use?" \
  --decision "Sequential prefixed IDs: task-1, task-2, etc." \
  --rationale "Human-readable, easy to reference in conversation, simple for in-memory store" \
  --alternative "UUIDs|Globally unique but hard to remember/type in CLI contexts" \
  --alternative "Nanoids|Shorter than UUID but still not human-friendly" \
  --tag "tasks" \
  --tag "data-model"

# --- Git commit ---
git add -A
git commit -m "$(cat <<'EOF'
Implement task CRUD with validation constraints

Task IDs use sequential prefixed format (task-1, task-2) for
human readability, over UUIDs (hard to type) and nanoids.

Validation rules:
- Task must reference valid board_id
- Title: non-empty, <= 200 chars
- Modification: only assignee or board owner

Behavior specs:
- Valid board_id → new task with Todo status
- Invalid board_id → 400
- Existing task → return it; missing → 404
EOF
)"

# --- Intent 2: Task status transitions ---
INTENT2=$($TELOS_BIN intent \
  --statement "Define task status transition rules" \
  --constraint "Status transitions must follow: Todo -> InProgress -> Done" \
  --constraint "Cannot transition backwards (Done -> Todo) without explicit reset" \
  --impact "tasks" \
  --behavior "GIVEN a task with status Todo|WHEN status is set to InProgress|THEN update succeeds" \
  --behavior "GIVEN a task with status Done|WHEN status is set to Todo|THEN return error: backward transition not allowed" \
  2>&1 | grep -o '\[.*\] [a-f0-9]*' | awk '{print $2}')

echo "Status transitions intent: $INTENT2"

git add -A
git commit -m "$(cat <<'EOF'
Define task status transition rules

Forward-only transitions: Todo → InProgress → Done.
Backward transitions require explicit reset operation.
This prevents accidental regression of completed work.
EOF
)"

echo "=== Stage 3 complete: 2 intents, 1 decision, 2 git commits ==="
