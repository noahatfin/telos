use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A SHA-256 content address, displayed and stored as 64 hex chars.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ObjectId(String);

impl ObjectId {
    /// Create an ObjectId by hashing raw bytes.
    pub fn hash(data: &[u8]) -> Self {
        let digest = Sha256::digest(data);
        Self(hex::encode(digest))
    }

    /// Parse a full 64-char hex string into an ObjectId.
    pub fn parse(hex_str: &str) -> Result<Self, crate::error::CoreError> {
        let hex_str = hex_str.trim();
        if hex_str.len() != 64 || !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(crate::error::CoreError::InvalidObjectId(
                hex_str.to_string(),
            ));
        }
        Ok(Self(hex_str.to_lowercase()))
    }

    /// The full 64-char hex representation.
    pub fn hex(&self) -> &str {
        &self.0
    }

    /// First 8 chars, used for display.
    pub fn short(&self) -> &str {
        &self.0[..8]
    }

    /// First 2 hex chars — used as fan-out directory name.
    pub fn fan_out(&self) -> (&str, &str) {
        (&self.0[..2], &self.0[2..])
    }

    /// Check if this ObjectId starts with the given prefix.
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", self.short())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let a = ObjectId::hash(b"hello world");
        let b = ObjectId::hash(b"hello world");
        assert_eq!(a, b);
        assert_eq!(a.hex().len(), 64);
    }

    #[test]
    fn hash_different_inputs() {
        let a = ObjectId::hash(b"hello");
        let b = ObjectId::hash(b"world");
        assert_ne!(a, b);
    }

    #[test]
    fn parse_valid() {
        let id = ObjectId::hash(b"test");
        let parsed = ObjectId::parse(id.hex()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_invalid_length() {
        assert!(ObjectId::parse("abcd").is_err());
    }

    #[test]
    fn parse_invalid_chars() {
        let bad = "g".repeat(64);
        assert!(ObjectId::parse(&bad).is_err());
    }

    #[test]
    fn fan_out_split() {
        let id = ObjectId::hash(b"test");
        let (dir, file) = id.fan_out();
        assert_eq!(dir.len(), 2);
        assert_eq!(file.len(), 62);
        assert_eq!(format!("{}{}", dir, file), id.hex());
    }

    #[test]
    fn display_short() {
        let id = ObjectId::hash(b"test");
        let display = format!("{}", id);
        assert_eq!(display.len(), 8);
        assert_eq!(display, id.short());
    }
}
