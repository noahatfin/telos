use serde::{Deserialize, Serialize};

/// JWT token configuration
pub const TOKEN_EXPIRY_SECS: u64 = 3600; // 1 hour — CONSTRAINT: must be <= 1 hour

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
    // Simplified validation for demo purposes
    if token.is_empty() {
        return Err(AuthError::EmptyToken);
    }
    if !token.starts_with("tb_") {
        return Err(AuthError::InvalidFormat);
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
    #[error("empty token")]
    EmptyToken,
    #[error("invalid token format")]
    InvalidFormat,
    #[error("token expired")]
    Expired,
    #[error("invalid signature")]
    InvalidSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_expiry_within_limit() {
        // This test enforces the constraint: token expiry <= 1 hour
        assert!(TOKEN_EXPIRY_SECS <= 3600, "Token expiry must be <= 1 hour (3600 seconds)");
    }

    #[test]
    fn validate_empty_token_fails() {
        let config = AuthConfig::default();
        assert!(validate_token("", &config).is_err());
    }

    #[test]
    fn validate_invalid_format_fails() {
        let config = AuthConfig::default();
        assert!(validate_token("bad-token", &config).is_err());
    }

    #[test]
    fn validate_valid_format_succeeds() {
        let config = AuthConfig::default();
        assert!(validate_token("tb_test123", &config).is_ok());
    }
}
