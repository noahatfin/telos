#!/usr/bin/env bash
# Stage 7: Status transition regression — two-phase bug introduction
#
# Phase 1: Add proper forward-only transition validation (good commit)
# Phase 2: Remove it with "flexibility" justification (the regression)
#
# Telos constraint violated: "Cannot transition backwards without explicit reset"
# (recorded in Stage 3 via 03_tasks.sh)
#
# Usage: ./07_status_regression.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 7: Status Transition Regression ==="
cd "$PROJECT_DIR"

# --- Phase 1: Add forward-only transition validation (the CORRECT implementation) ---
cat > src/tasks/mod.rs <<'RUSTEOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub board_id: String,
    pub assignee: Option<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub board_id: String,
    pub assignee: Option<String>,
}

/// In-memory task store (for validation purposes)
pub struct TaskStore {
    tasks: Vec<Task>,
    next_id: u32,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new task.
    /// CONSTRAINT: Task must reference a valid board_id (caller must verify)
    pub fn create(&mut self, req: CreateTaskRequest) -> Task {
        let task = Task {
            id: format!("task-{}", self.next_id),
            title: req.title,
            description: req.description,
            board_id: req.board_id,
            assignee: req.assignee,
            status: TaskStatus::Todo,
        };
        self.next_id += 1;
        self.tasks.push(task.clone());
        task
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn list_by_board(&self, board_id: &str) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.board_id == board_id).collect()
    }

    /// Update task status with forward-only transition enforcement.
    /// CONSTRAINT: Status transitions must follow Todo -> InProgress -> Done.
    /// Cannot transition backwards without explicit reset.
    pub fn update_status(&mut self, id: &str, status: TaskStatus) -> Option<&Task> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            // Enforce forward-only transitions
            let allowed = match (&task.status, &status) {
                (TaskStatus::Todo, TaskStatus::InProgress) => true,
                (TaskStatus::InProgress, TaskStatus::Done) => true,
                (s, t) if s == t => true, // no-op is always allowed
                _ => false, // backward transitions not allowed
            };
            if !allowed {
                return None;
            }
            task.status = status;
            Some(task)
        } else {
            None
        }
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_task() {
        let mut store = TaskStore::new();
        let task = store.create(CreateTaskRequest {
            title: "Test task".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        assert_eq!(store.get(&task.id).unwrap().title, "Test task");
    }

    #[test]
    fn list_by_board() {
        let mut store = TaskStore::new();
        store.create(CreateTaskRequest {
            title: "Task A".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        store.create(CreateTaskRequest {
            title: "Task B".into(),
            description: None,
            board_id: "board-2".into(),
            assignee: None,
        });
        assert_eq!(store.list_by_board("board-1").len(), 1);
    }

    #[test]
    fn forward_transition_allowed() {
        let mut store = TaskStore::new();
        let task = store.create(CreateTaskRequest {
            title: "Task".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        // Todo -> InProgress: allowed
        assert!(store.update_status(&task.id, TaskStatus::InProgress).is_some());
        // InProgress -> Done: allowed
        assert!(store.update_status(&task.id, TaskStatus::Done).is_some());
        assert_eq!(store.get(&task.id).unwrap().status, TaskStatus::Done);
    }

    #[test]
    fn backward_transition_blocked() {
        let mut store = TaskStore::new();
        let task = store.create(CreateTaskRequest {
            title: "Task".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        store.update_status(&task.id, TaskStatus::InProgress);
        store.update_status(&task.id, TaskStatus::Done);
        // Done -> Todo: NOT allowed
        assert!(store.update_status(&task.id, TaskStatus::Todo).is_none());
        assert_eq!(store.get(&task.id).unwrap().status, TaskStatus::Done);
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Add forward-only task status transition validation

Enforces the transition rule: Todo → InProgress → Done.
Backward transitions (e.g., Done → Todo) are now rejected
to prevent accidental regression of completed work.

Adds tests for both valid forward transitions and blocked
backward transitions.
EOF
)"

# --- Phase 2: Remove the validation (THE REGRESSION) ---
cat > src/tasks/mod.rs <<'RUSTEOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub board_id: String,
    pub assignee: Option<String>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub board_id: String,
    pub assignee: Option<String>,
}

/// In-memory task store (for validation purposes)
pub struct TaskStore {
    tasks: Vec<Task>,
    next_id: u32,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new task.
    /// CONSTRAINT: Task must reference a valid board_id (caller must verify)
    pub fn create(&mut self, req: CreateTaskRequest) -> Task {
        let task = Task {
            id: format!("task-{}", self.next_id),
            title: req.title,
            description: req.description,
            board_id: req.board_id,
            assignee: req.assignee,
            status: TaskStatus::Todo,
        };
        self.next_id += 1;
        self.tasks.push(task.clone());
        task
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn list_by_board(&self, board_id: &str) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.board_id == board_id).collect()
    }

    pub fn update_status(&mut self, id: &str, status: TaskStatus) -> Option<&Task> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = status;
            Some(task)
        } else {
            None
        }
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_get_task() {
        let mut store = TaskStore::new();
        let task = store.create(CreateTaskRequest {
            title: "Test task".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        assert_eq!(store.get(&task.id).unwrap().title, "Test task");
    }

    #[test]
    fn list_by_board() {
        let mut store = TaskStore::new();
        store.create(CreateTaskRequest {
            title: "Task A".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        store.create(CreateTaskRequest {
            title: "Task B".into(),
            description: None,
            board_id: "board-2".into(),
            assignee: None,
        });
        assert_eq!(store.list_by_board("board-1").len(), 1);
    }

    #[test]
    fn update_status() {
        let mut store = TaskStore::new();
        let task = store.create(CreateTaskRequest {
            title: "Task".into(),
            description: None,
            board_id: "board-1".into(),
            assignee: None,
        });
        store.update_status(&task.id, TaskStatus::Done);
        assert_eq!(store.get(&task.id).unwrap().status, TaskStatus::Done);
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Allow flexible task status updates for better workflow

Removed rigid status transition checks that were blocking
legitimate workflows. Users need to move tasks freely
between statuses without artificial restrictions.

Simplifies the update_status logic and restores the
straightforward status update behavior.
EOF
)"

echo ""
echo "=== Stage 7 complete ==="
echo "Two-phase regression introduced:"
echo "  Phase 1: Added forward-only transition validation (correct)"
echo "  Phase 2: Removed it for 'flexibility' (violates constraint)"
echo ""
echo "Telos constraint violated: 'Cannot transition backwards without explicit reset'"
echo "This regression is used by Experiment E."
