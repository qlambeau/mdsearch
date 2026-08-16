use std::fmt::Write;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// A SHA-256 content hash represented as lowercase hexadecimal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentHash(String);

/// Describes why a content hash string is invalid.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ContentHashError {
    /// The value is not a 64-character hexadecimal string.
    #[error("content hash must be 64 hexadecimal characters")]
    Invalid,
}

impl ContentHash {
    /// Computes the SHA-256 hash of `content`.
    #[must_use]
    pub fn from_content(content: &[u8]) -> Self {
        let digest = Sha256::digest(content);
        let hex = digest
            .iter()
            .fold(String::with_capacity(64), |mut output, byte| {
                // Writing to a `String` cannot fail.
                let _ = write!(output, "{byte:02x}");
                output
            });
        Self(hex)
    }

    /// Reconstructs a hash from a lowercase hexadecimal string.
    ///
    /// # Errors
    ///
    /// Returns an error when the string is not 64 hexadecimal characters.
    pub fn try_from_hex(hex: &str) -> Result<Self, ContentHashError> {
        if hex.len() == 64 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
            Ok(Self(hex.to_ascii_lowercase()))
        } else {
            Err(ContentHashError::Invalid)
        }
    }

    /// Returns the hash as a lowercase hexadecimal string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::ContentHash;

    /// Covers: FR-010 — identical content yields identical hashes.
    #[test]
    fn hashes_identical_content_identically() {
        let first = ContentHash::from_content(b"same content");
        let second = ContentHash::from_content(b"same content");

        assert_eq!(first, second);
    }

    /// Covers: FR-010 — differing content yields differing hashes.
    #[test]
    fn hashes_differing_content_differently() {
        let first = ContentHash::from_content(b"first");
        let second = ContentHash::from_content(b"second");

        assert_ne!(first, second);
    }

    /// Covers: the hash is lowercase hexadecimal of the SHA-256 digest length.
    #[test]
    fn produces_a_64_character_lowercase_hex_string() {
        let hash = ContentHash::from_content(b"content");

        let hex = hash.as_str();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|character| character.is_ascii_hexdigit()));
        assert_eq!(hex, hex.to_lowercase());
    }

    /// Covers: `try_from_hex` reconstructs a hash that round-trips with
    /// `from_content`.
    #[test]
    fn reconstructs_a_hash_from_hex() -> Result<(), super::ContentHashError> {
        let hash = ContentHash::from_content(b"content");

        let reconstructed = ContentHash::try_from_hex(hash.as_str())?;

        assert_eq!(reconstructed, hash);

        Ok(())
    }

    /// Covers: `try_from_hex` rejects malformed input.
    #[test]
    fn rejects_invalid_hex() {
        assert!(ContentHash::try_from_hex("too short").is_err());
        assert!(ContentHash::try_from_hex(&"g".repeat(64)).is_err());
    }

    proptest! {
        /// Covers: FR-010 — hashing is deterministic for arbitrary content.
        #[test]
        fn is_deterministic_for_arbitrary_content(content in proptest::collection::vec(any::<u8>(), 0..1024)) {
            let first = ContentHash::from_content(&content);
            let second = ContentHash::from_content(&content);

            prop_assert_eq!(first, second);
        }
    }
}
