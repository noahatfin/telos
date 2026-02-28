#!/usr/bin/env bash
# Stage 9: Benign changes for false-positive testing
#
# Creates commits that look similar to regressions but violate NO constraints.
# Used by Experiments N and O to test for false positives.
#
# Change 1: Benign refactor — renames variables and reorganizes functions in auth module
# Change 2: Near-miss — changes TOKEN_EXPIRY_SECS from 3600 to 3500 (still within 1-hour constraint)
#
# Usage: ./09_benign_changes.sh <project_dir>
set -euo pipefail

PROJECT_DIR="${1:?Usage: $0 <project_dir>}"
TELOS_BIN="${TELOS_BIN:-telos}"

export TELOS_AUTHOR_NAME="Agent"
export TELOS_AUTHOR_EMAIL="agent@taskboard.dev"

echo "=== Stage 9: Benign Changes (False-Positive Testing) ==="
cd "$PROJECT_DIR"

# --- Save current state to restore later ---
CURRENT_HEAD=$(git rev-parse HEAD)

# --- Change 1: Benign refactor of auth module ---
# Reset to the state after stage 4 (clean, before regressions)
# Find the commit "Add board management module" which ends stage 4
STAGE4_COMMIT=$(git log --oneline --all --grep="Add board management" | head -1 | awk '{print $1}')
if [ -z "$STAGE4_COMMIT" ]; then
    # Fallback: use 4th commit from end of initial good state
    STAGE4_COMMIT=$(git log --oneline | tail -4 | head -1 | awk '{print $1}')
fi

git checkout -b benign-refactor "$STAGE4_COMMIT" 2>/dev/null || git checkout benign-refactor 2>/dev/null || true

# Refactor: rename variables for clarity, reorganize imports, add doc comments
# This is purely cosmetic — no behavior change
cat > src/auth/refactor_benign.rs <<'RUSTEOF'
// Benign refactor: Renamed variables for clarity
// TOKEN_EXPIRY_SECS remains 3600 (within constraint)
// Functions reordered alphabetically
// Added doc comments

use serde::{Deserialize, Serialize};

/// JWT token configuration — controls authentication behavior.
pub const TOKEN_EXPIRY_SECS: u64 = 3600; // 1 hour — CONSTRAINT: must be <= 1 hour

/// Authentication configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// The signing secret for JWT tokens.
    pub jwt_signing_secret: String,  // renamed from 'secret' for clarity
    /// Token lifetime in seconds.
    pub token_lifetime_secs: u64,    // renamed from 'token_expiry_secs' for clarity
    /// The issuer claim for JWT tokens.
    pub token_issuer: String,        // renamed from 'issuer' for clarity
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_signing_secret: "dev-secret-do-not-use-in-prod".into(),
            token_lifetime_secs: TOKEN_EXPIRY_SECS,
            token_issuer: "taskboard".into(),
        }
    }
}

/// User claims embedded in JWT tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {  // renamed from 'Claims' for clarity
    pub subject: String,  // renamed from 'sub'
    pub expires_at: u64,  // renamed from 'exp'
    pub issuer: String,   // renamed from 'iss'
    pub user_role: UserRole,  // renamed from 'role'
}

/// Role-based access control roles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Member,
    Viewer,
}

/// Validate a JWT token and return claims.
/// Returns Member role as default (satisfies constraint: never defaults to Admin).
pub fn validate_token(token: &str, config: &AuthConfig) -> Result<TokenClaims, AuthError> {
    if token.is_empty() {
        return Err(AuthError::EmptyToken);
    }
    if !token.starts_with("tb_") {
        return Err(AuthError::InvalidFormat);
    }
    Ok(TokenClaims {
        subject: "user-1".into(),
        expires_at: 0,
        issuer: config.token_issuer.clone(),
        user_role: UserRole::Member,
    })
}

/// Authentication error types — generic messages only (no internal details).
#[derive(Debug)]
pub enum AuthError {
    EmptyToken,
    InvalidFormat,
    Expired,
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_token_fails() {
        let config = AuthConfig::default();
        assert!(validate_token("", &config).is_err());
    }

    #[test]
    fn validate_valid_format_succeeds() {
        let config = AuthConfig::default();
        let claims = validate_token("tb_test123", &config).unwrap();
        assert_eq!(claims.user_role, UserRole::Member);
    }

    #[test]
    fn token_expiry_within_limit() {
        assert!(TOKEN_EXPIRY_SECS <= 3600, "Token expiry must be <= 1 hour");
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Refactor auth module for clarity

Renamed variables for better readability:
- secret -> jwt_signing_secret
- token_expiry_secs -> token_lifetime_secs
- Claims -> TokenClaims
- sub/exp/iss -> subject/expires_at/issuer

Reordered functions alphabetically and added doc comments.
No behavioral changes — all constraints still satisfied.
EOF
)"

# --- Change 2: Near-miss token expiry adjustment ---
git checkout -b near-miss-change "$STAGE4_COMMIT" 2>/dev/null || git checkout near-miss-change 2>/dev/null || true

# Change TOKEN_EXPIRY_SECS from 3600 to 3500 — still within the <= 1 hour constraint
cat > src/auth/near_miss.rs <<'RUSTEOF'
use serde::{Deserialize, Serialize};

/// JWT token configuration
pub const TOKEN_EXPIRY_SECS: u64 = 3500; // ~58 minutes — within 1 hour constraint

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
            token_expiry_secs: TOKEN_EXPIRY_SECS,
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
    if token.is_empty() {
        return Err(AuthError::EmptyToken);
    }
    if !token.starts_with("tb_") {
        return Err(AuthError::InvalidFormat);
    }
    Ok(Claims {
        sub: "user-1".into(),
        exp: 0,
        iss: config.issuer.clone(),
        role: UserRole::Member,
    })
}

#[derive(Debug)]
pub enum AuthError {
    EmptyToken,
    InvalidFormat,
    Expired,
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_token_fails() {
        let config = AuthConfig::default();
        assert!(validate_token("", &config).is_err());
    }

    #[test]
    fn validate_valid_format_succeeds() {
        let config = AuthConfig::default();
        let claims = validate_token("tb_test123", &config).unwrap();
        assert_eq!(claims.role, UserRole::Member);
    }

    #[test]
    fn token_expiry_within_limit() {
        assert!(TOKEN_EXPIRY_SECS <= 3600, "Token expiry must be <= 1 hour");
    }
}
RUSTEOF

git add -A
git commit -m "$(cat <<'EOF'
Reduce token expiry to 58 minutes for tighter security

Lowered TOKEN_EXPIRY_SECS from 3600 to 3500 (~58 minutes)
to provide a small buffer below the 1-hour constraint limit.
This helps ensure tokens expire before the hard limit even
with minor clock skew between servers.
EOF
)"

# Return to original branch
git checkout - 2>/dev/null || git checkout "$CURRENT_HEAD" 2>/dev/null

echo ""
echo "=== Stage 9 complete ==="
echo "Benign changes created on separate branches:"
echo "  Branch 'benign-refactor': Variable renames, doc comments (no violations)"
echo "  Branch 'near-miss-change': Token expiry 3600 -> 3500 (still within constraint)"
echo ""
echo "These are used by false-positive Experiments N and O."
