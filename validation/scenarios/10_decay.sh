#!/usr/bin/env bash
# Stage 10: Intent Decay — code evolves without updating Telos intents
#
# Starts from a clean state after stage 4, then makes 20 commits
# that significantly evolve the codebase:
# - Renames modules and functions
# - Refactors data structures
# - Adds new modules with different terminology
# - Changes API contracts
#
# The key: Telos intents are NOT updated. They become stale,
# referencing old code concepts that no longer exist.
#
# Usage: ./10_decay.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 10: Intent Decay (20 evolving commits, no intent updates) ==="
cd "$PROJECT_DIR"

# Save current state
CURRENT_HEAD=$(git rev-parse HEAD)

# Find the clean state after stage 4
STAGE4_COMMIT=$(git log --oneline --all --grep="Add board management" | head -1 | awk '{print $1}')
if [ -z "$STAGE4_COMMIT" ]; then
    STAGE4_COMMIT=$(git log --oneline | tail -4 | head -1 | awk '{print $1}')
fi

git checkout -b decay-test "$STAGE4_COMMIT" 2>/dev/null || git checkout decay-test 2>/dev/null || true

# --- Commit 1: Rename auth module to identity ---
mkdir -p src/identity
cp src/auth/mod.rs src/identity/mod.rs 2>/dev/null || true
sed -i '' 's/AuthConfig/IdentityConfig/g' src/identity/mod.rs 2>/dev/null || true
git add -A && git commit -m "Rename auth module to identity" -q

# --- Commit 2: Rename Tasks to WorkItems ---
if [ -f src/tasks/mod.rs ]; then
    mkdir -p src/work_items
    cp src/tasks/mod.rs src/work_items/mod.rs
    sed -i '' 's/Task/WorkItem/g; s/task/work_item/g' src/work_items/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename tasks module to work_items" -q
fi

# --- Commit 3: Change JWT to session-based auth ---
cat > src/identity/session.rs <<'EOF'
pub struct SessionToken {
    pub session_id: String,
    pub user_id: String,
    pub created_at: u64,
    pub max_age_secs: u64,
}

pub fn create_session(user_id: &str) -> SessionToken {
    SessionToken {
        session_id: format!("sess_{}", user_id),
        user_id: user_id.to_string(),
        created_at: 0,
        max_age_secs: 7200, // 2 hours for sessions
    }
}
EOF
git add -A && git commit -m "Add session-based auth alongside JWT" -q

# --- Commit 4: Rename Board to Workspace ---
if [ -f src/boards/mod.rs ]; then
    mkdir -p src/workspaces
    cp src/boards/mod.rs src/workspaces/mod.rs
    sed -i '' 's/Board/Workspace/g; s/board/workspace/g' src/workspaces/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename boards to workspaces" -q
fi

# --- Commit 5: Add API versioning layer ---
mkdir -p src/api/v2
cat > src/api/v2/mod.rs <<'EOF'
pub mod routes {
    pub fn health() -> &'static str { "ok" }
    pub fn version() -> &'static str { "v2" }
}
EOF
git add -A && git commit -m "Add API v2 routing layer" -q

# --- Commit 6: Replace UserRole with PermissionSet ---
cat > src/identity/permissions.rs <<'EOF'
use std::collections::HashSet;

pub struct PermissionSet {
    pub permissions: HashSet<String>,
}

impl PermissionSet {
    pub fn admin() -> Self {
        let mut perms = HashSet::new();
        perms.insert("read".into());
        perms.insert("write".into());
        perms.insert("delete".into());
        perms.insert("admin".into());
        Self { permissions: perms }
    }
    pub fn member() -> Self {
        let mut perms = HashSet::new();
        perms.insert("read".into());
        perms.insert("write".into());
        Self { permissions: perms }
    }
}
EOF
git add -A && git commit -m "Replace UserRole enum with PermissionSet" -q

# --- Commit 7: Add caching layer ---
cat > src/cache.rs <<'EOF'
use std::collections::HashMap;
pub struct Cache<V> {
    store: HashMap<String, V>,
    max_entries: usize,
}
impl<V> Cache<V> {
    pub fn new(max_entries: usize) -> Self {
        Self { store: HashMap::new(), max_entries }
    }
}
EOF
git add -A && git commit -m "Add in-memory cache layer" -q

# --- Commit 8: Rename validate_token to authenticate ---
if [ -f src/identity/mod.rs ]; then
    sed -i '' 's/validate_token/authenticate/g' src/identity/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename validate_token to authenticate" -q
fi

# --- Commit 9: Add event sourcing for work items ---
cat > src/work_items/events.rs <<'EOF'
pub enum WorkItemEvent {
    Created { id: String, title: String },
    StatusChanged { id: String, from: String, to: String },
    Assigned { id: String, assignee: String },
    Archived { id: String },
}
EOF
git add -A && git commit -m "Add event sourcing for work items" -q

# --- Commit 10: Rename secret to signing_key ---
if [ -f src/identity/mod.rs ]; then
    sed -i '' 's/secret/signing_key/g' src/identity/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename secret field to signing_key" -q
fi

# --- Commit 11: Add middleware layer ---
cat > src/middleware.rs <<'EOF'
pub struct RequestContext {
    pub trace_id: String,
    pub user_id: Option<String>,
    pub permissions: Vec<String>,
}
EOF
git add -A && git commit -m "Add request middleware with trace context" -q

# --- Commit 12: Replace in-memory store with repository pattern ---
cat > src/work_items/repository.rs <<'EOF'
pub trait WorkItemRepository {
    fn find_by_id(&self, id: &str) -> Option<String>;
    fn find_by_workspace(&self, workspace_id: &str) -> Vec<String>;
    fn save(&mut self, item: String) -> Result<(), String>;
    fn delete(&mut self, id: &str) -> Result<(), String>;
}
EOF
git add -A && git commit -m "Add repository pattern for work items" -q

# --- Commit 13: Rename Claims to AuthPayload ---
if [ -f src/identity/mod.rs ]; then
    sed -i '' 's/Claims/AuthPayload/g' src/identity/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename Claims struct to AuthPayload" -q
fi

# --- Commit 14: Add notification service ---
cat > src/notifications.rs <<'EOF'
pub struct Notification {
    pub recipient: String,
    pub message: String,
    pub channel: NotificationChannel,
}
pub enum NotificationChannel { Email, InApp, Webhook }
EOF
git add -A && git commit -m "Add notification service module" -q

# --- Commit 15: Restructure error types ---
cat > src/errors.rs <<'EOF'
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    Internal(String),
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "not found: {}", msg),
            Self::Unauthorized(msg) => write!(f, "unauthorized: {}", msg),
            Self::Forbidden(msg) => write!(f, "forbidden: {}", msg),
            Self::Conflict(msg) => write!(f, "conflict: {}", msg),
            Self::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}
EOF
git add -A && git commit -m "Restructure error types into unified AppError" -q

# --- Commit 16: Replace TOKEN_EXPIRY_SECS with config-based duration ---
if [ -f src/identity/mod.rs ]; then
    sed -i '' 's/TOKEN_EXPIRY_SECS/SESSION_DURATION_SECS/g' src/identity/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Replace TOKEN_EXPIRY_SECS with configurable session duration" -q
fi

# --- Commit 17: Add health check module ---
cat > src/health.rs <<'EOF'
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}
pub fn check() -> HealthStatus {
    HealthStatus { status: "healthy".into(), version: "2.0.0".into(), uptime_secs: 0 }
}
EOF
git add -A && git commit -m "Add health check endpoint" -q

# --- Commit 18: Rename CreateBoardRequest to CreateWorkspaceInput ---
if [ -f src/workspaces/mod.rs ]; then
    sed -i '' 's/CreateBoardRequest/CreateWorkspaceInput/g' src/workspaces/mod.rs 2>/dev/null || true
    git add -A && git commit -m "Rename CreateBoardRequest to CreateWorkspaceInput" -q
fi

# --- Commit 19: Add GraphQL schema ---
cat > src/api/v2/schema.rs <<'EOF'
pub const SCHEMA: &str = r#"
type Query {
    workspace(id: ID!): Workspace
    workItem(id: ID!): WorkItem
    me: User
}
type Workspace { id: ID!, name: String!, items: [WorkItem!]! }
type WorkItem { id: ID!, title: String!, status: String! }
type User { id: ID!, permissions: [String!]! }
"#;
EOF
git add -A && git commit -m "Add GraphQL schema for v2 API" -q

# --- Commit 20: Remove old module stubs ---
rm -rf src/tasks 2>/dev/null || true
rm -rf src/boards 2>/dev/null || true
git add -A && git commit -m "Remove deprecated tasks and boards modules" -q

# Return to original branch
git checkout - 2>/dev/null || git checkout "$CURRENT_HEAD" 2>/dev/null || true

echo ""
echo "=== Stage 10 complete ==="
echo "20 commits on 'decay-test' branch evolving the codebase:"
echo "  - auth -> identity (module rename)"
echo "  - tasks -> work_items (module rename)"
echo "  - boards -> workspaces (module rename)"
echo "  - Claims -> AuthPayload (struct rename)"
echo "  - validate_token -> authenticate (function rename)"
echo "  - TOKEN_EXPIRY_SECS -> SESSION_DURATION_SECS (constant rename)"
echo "  - UserRole -> PermissionSet (enum to struct)"
echo "  - Added: sessions, events, cache, middleware, notifications, GraphQL"
echo "  - Removed: old tasks/ and boards/ directories"
echo ""
echo "Telos intents were NOT updated — they still reference:"
echo "  - 'auth' module, 'Tasks', 'Board', 'Claims', 'validate_token'"
echo "  - TOKEN_EXPIRY_SECS, UserRole enum, etc."
echo ""
echo "This decay scenario is used by Experiment K."
