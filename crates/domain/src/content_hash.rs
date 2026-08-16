use std::fmt::Write;

use sha2::{Digest, Sha256};

/// A SHA-256 content hash represented as lowercase hexadecimal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentHash(String);

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
