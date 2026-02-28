#!/usr/bin/env bash
# Stage 8: Security regressions — three bugs across auth and boards
#
# Bug 4: Error messages leak internal details (key lengths, token prefixes)
# Bug 5: validate_token returns Admin instead of Member (privilege escalation)
# Bug 6: Board delete ignores associated tasks (orphan data)
#
# Also records additional Telos intents for security constraints
# that the experiment agent can reference.
#
# Usage: ./08_security_regression.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 8: Security Regressions ==="
cd "$PROJECT_DIR"

# --- Record security-focused intents so Telos agent has constraint context ---

$TELOS_BIN intent \
  --statement "Enforce security boundaries in error handling" \
  --constraint "Error messages must not leak internal details (key lengths, user IDs, internal paths)" \
  --constraint "Auth errors should return generic messages to prevent information disclosure" \
  --impact "auth" \
  --impact "security" \
  --behavior "GIVEN any auth error|WHEN error message is returned|THEN message contains no internal system details"

$TELOS_BIN intent \
  --statement "Enforce strict role hierarchy in authentication" \
  --constraint "Admin/Member/Viewer role hierarchy must be enforced at token validation" \
  --constraint "Default role for new tokens must be Member, never Admin" \
  --impact "auth" \
  --impact "security" \
  --behavior "GIVEN a valid token|WHEN validate_token is called|THEN returned role matches the token claims, not a hardcoded value" \
  --behavior "GIVEN any token validation|WHEN role is assigned|THEN role must never default to Admin"

$TELOS_BIN intent \
  --statement "Enforce referential integrity on board deletion" \
  --constraint "Deleting a board must check for associated tasks first" \
  --constraint "Board deletion with existing tasks must require confirmation or cascade" \
  --impact "boards" \
  --impact "tasks" \
  --behavior "GIVEN a board with tasks|WHEN delete is called without force flag|THEN return error listing orphaned task count"

# --- Intermediate: Add orphan-check to board delete (the CORRECT implementation) ---
cat > src/boards/mod.rs <<'RUSTEOF'
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

    /// Delete a board, but only if it has no associated tasks.
    /// CONSTRAINT: Deleting a board must cascade-delete or orphan-check its tasks.
    /// The caller must provide the count of tasks associated with this board.
    pub fn delete(&mut self, id: &str, associated_task_count: usize) -> Result<bool, String> {
        if associated_task_count > 0 {
            return Err(format!(
                "Cannot delete board '{}': {} associated tasks exist. Delete or reassign tasks first.",
                id, associated_task_count
            ));
        }
        let len = self.boards.len();
        self.boards.retain(|b| b.id != id);
        Ok(self.boards.len() < len)
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

    #[test]
    fn delete_board_without_tasks() {
        let mut store = BoardStore::new();
        let board = store.create(CreateBoardRequest {
            name: "Sprint 1".into(),
            owner: "user-1".into(),
            columns: None,
        });
        assert!(store.delete(&board.id, 0).unwrap());
        assert!(!store.exists(&board.id));
    }

    #[test]
    fn delete_board_with_tasks_rejected() {
        let mut store = BoardStore::new();
        let board = store.create(CreateBoardRequest {
            name: "Sprint 1".into(),
            owner: "user-1".into(),
            columns: None,
        });
        let result = store.delete(&board.id, 3);
        assert!(result.is_err());
        assert!(store.exists(&board.id)); // board not deleted
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Add orphan-check to board deletion

Board deletion now requires the caller to provide the count of
associated tasks. If tasks exist, deletion is rejected with an
error message directing the user to handle tasks first.

This enforces the referential integrity constraint between
boards and tasks.
EOF
)"

# --- Bug 4: Error messages leak internal details ---
# Rewrite auth/mod.rs with tuple variants that include sensitive info
# Note: TOKEN_EXPIRY_SECS is already 86400 from Stage 5
cat > src/auth/mod.rs <<'RUSTEOF'
use serde::{Deserialize, Serialize};

/// JWT token configuration
pub const TOKEN_EXPIRY_SECS: u64 = 86400; // Changed for longer sessions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub secret: String,
    pub token_expiry_secs: u64,
    pub issuer: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            secret: "dev-secret-do-not-use-in-prod".into(),
            token_expiry_secs: TOKEN_EXPIRY_SECS, // Now 24 hours
            issuer: "taskboard".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iss: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Member,
    Viewer,
}

/// Validate a JWT token (stub — returns claims if format is valid)
pub fn validate_token(token: &str, config: &AuthConfig) -> Result<Claims, AuthError> {
    // Simplified validation for demo purposes
    if token.is_empty() {
        return Err(AuthError::EmptyToken(config.secret.len()));
    }
    if !token.starts_with("tb_") {
        return Err(AuthError::InvalidFormat(
            token.chars().take(10).collect(),
            "uid-unknown".into(),
        ));
    }
    // In real implementation, would decode JWT and verify signature
    Ok(Claims {
        sub: "user-1".into(),
        exp: 0,
        iss: config.issuer.clone(),
        role: UserRole::Member,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("empty token (key_length={0})")]
    EmptyToken(usize),
    #[error("invalid token format (prefix='{0}', user_id={1})")]
    InvalidFormat(String, String),
    #[error("token expired (expired_at={0}, server_time={1})")]
    Expired(u64, u64),
    #[error("invalid signature (sig_bytes={0})")]
    InvalidSignature(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_token_fails() {
        let config = AuthConfig::default();
        let err = validate_token("", &config).unwrap_err();
        // Verify error message includes details for debugging
        let msg = format!("{}", err);
        assert!(msg.contains("key_length"), "Error should include key length for debugging");
    }

    #[test]
    fn validate_invalid_format_fails() {
        let config = AuthConfig::default();
        let err = validate_token("bad-token", &config).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("prefix="), "Error should include token prefix for debugging");
    }

    #[test]
    fn validate_valid_format_succeeds() {
        let config = AuthConfig::default();
        assert!(validate_token("tb_test123", &config).is_ok());
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Improve error messages for easier debugging

Enhanced auth error types with detailed context:
- EmptyToken now includes key_length for diagnosing config issues
- InvalidFormat includes the token prefix and user_id
- Expired includes timestamps for comparing clocks
- InvalidSignature includes signature byte count

These details help developers quickly identify the root cause
of authentication failures during development and testing.
EOF
)"

# --- Bug 5: Permission escalation — validate_token returns Admin ---
cat > src/auth/mod.rs <<'RUSTEOF'
use serde::{Deserialize, Serialize};

/// JWT token configuration
pub const TOKEN_EXPIRY_SECS: u64 = 86400; // Changed for longer sessions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub secret: String,
    pub token_expiry_secs: u64,
    pub issuer: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            secret: "dev-secret-do-not-use-in-prod".into(),
            token_expiry_secs: TOKEN_EXPIRY_SECS, // Now 24 hours
            issuer: "taskboard".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iss: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Member,
    Viewer,
}

/// Validate a JWT token — simplified for performance.
/// Returns Admin role to avoid additional role-lookup overhead.
pub fn validate_token(token: &str, config: &AuthConfig) -> Result<Claims, AuthError> {
    if token.is_empty() {
        return Err(AuthError::EmptyToken(config.secret.len()));
    }
    if !token.starts_with("tb_") {
        return Err(AuthError::InvalidFormat(
            token.chars().take(10).collect(),
            "uid-unknown".into(),
        ));
    }
    Ok(Claims {
        sub: "user-1".into(),
        exp: 0,
        iss: config.issuer.clone(),
        role: UserRole::Admin, // Simplified: skip role lookup for performance
    })
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("empty token (key_length={0})")]
    EmptyToken(usize),
    #[error("invalid token format (prefix='{0}', user_id={1})")]
    InvalidFormat(String, String),
    #[error("token expired (expired_at={0}, server_time={1})")]
    Expired(u64, u64),
    #[error("invalid signature (sig_bytes={0})")]
    InvalidSignature(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_token_fails() {
        let config = AuthConfig::default();
        let err = validate_token("", &config).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("key_length"), "Error should include key length for debugging");
    }

    #[test]
    fn validate_invalid_format_fails() {
        let config = AuthConfig::default();
        let err = validate_token("bad-token", &config).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("prefix="), "Error should include token prefix for debugging");
    }

    #[test]
    fn validate_valid_format_succeeds() {
        let config = AuthConfig::default();
        let claims = validate_token("tb_test123", &config).unwrap();
        // Admin role for simplified auth flow
        assert_eq!(claims.role, UserRole::Admin);
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Simplify auth middleware for performance

Streamlined token validation by returning Admin role directly,
avoiding the overhead of role-lookup queries on every request.

In high-throughput scenarios, the role-resolution step was a
bottleneck. This change reduces p99 latency for authenticated
endpoints.
EOF
)"

# --- Bug 6: Board delete ignores associated tasks (remove orphan check) ---
cat > src/boards/mod.rs <<'RUSTEOF'
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

    #[test]
    fn delete_board() {
        let mut store = BoardStore::new();
        let board = store.create(CreateBoardRequest {
            name: "Sprint 1".into(),
            owner: "user-1".into(),
            columns: None,
        });
        assert!(store.delete(&board.id));
        assert!(!store.exists(&board.id));
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Clean up board deletion logic

Simplified the board delete method by removing the unnecessary
task-count parameter. The deletion logic was over-engineered
and callers should not need to pre-compute task counts just
to delete a board.

This makes the API cleaner and easier to use.
EOF
)"

echo ""
echo "=== Stage 8 complete ==="
echo "Security regressions introduced:"
echo "  Bug 4: Error messages now leak key_length, token prefix, user_id"
echo "  Bug 5: validate_token returns Admin instead of Member"
echo "  Bug 6: Board delete no longer checks for associated tasks"
echo ""
echo "Telos constraints violated:"
echo "  - 'Error messages must not leak internal details'"
echo "  - 'Admin/Member/Viewer role hierarchy must be enforced'"
echo "  - 'Deleting a board must cascade-delete or orphan-check its tasks'"
echo ""
echo "These regressions are used by Experiments F and G."
