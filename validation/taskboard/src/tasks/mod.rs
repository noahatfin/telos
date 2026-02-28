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
