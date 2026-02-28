use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
    pub owner: String,
    pub columns: Option<Vec<String>>,
}

/// In-memory board store (for validation purposes)
pub struct BoardStore {
    boards: Vec<Board>,
    next_id: u32,
}

impl BoardStore {
    pub fn new() -> Self {
        Self {
            boards: Vec::new(),
            next_id: 1,
        }
    }

    pub fn create(&mut self, req: CreateBoardRequest) -> Board {
        let board = Board {
            id: format!("board-{}", self.next_id),
            name: req.name,
            owner: req.owner,
            columns: req.columns.unwrap_or_else(|| vec!["Todo".into(), "In Progress".into(), "Done".into()]),
        };
        self.next_id += 1;
        self.boards.push(board.clone());
        board
    }

    pub fn get(&self, id: &str) -> Option<&Board> {
        self.boards.iter().find(|b| b.id == id)
    }

    pub fn exists(&self, id: &str) -> bool {
        self.boards.iter().any(|b| b.id == id)
    }

    pub fn list(&self) -> &[Board] {
        &self.boards
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let len = self.boards.len();
        self.boards.retain(|b| b.id != id);
        self.boards.len() < len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_board_with_defaults() {
        let mut store = BoardStore::new();
        let board = store.create(CreateBoardRequest {
            name: "Sprint 1".into(),
            owner: "user-1".into(),
            columns: None,
        });
        assert_eq!(board.columns.len(), 3);
        assert!(store.exists(&board.id));
    }

    #[test]
    fn board_not_found() {
        let store = BoardStore::new();
        assert!(!store.exists("nonexistent"));
    }
}
