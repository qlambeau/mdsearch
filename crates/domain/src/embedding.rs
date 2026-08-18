//! Value types for the semantic (vector) index.

use std::path::Path;

use thiserror::Error;

use crate::content_hash::ContentHash;
use crate::file_id::FileId;
use crate::passage::PassageKind;
use crate::timestamp::Timestamp;

/// A local embedding model identifier.
///
/// The name is the canonical identifier of a model supported by the embedding
/// generator; the generator validates supportedness, so this type only enforces
/// that the name is non-empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingModel(String);

/// Describes why an embedding model name is invalid.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum EmbeddingModelError {
    /// The name is empty or whitespace-only.
    #[error("embedding model name must not be empty")]
    Empty,
}

impl EmbeddingModel {
    /// Creates a model from a non-empty name.
    ///
    /// # Errors
    ///
    /// Returns an empty error when the name is empty or whitespace-only.
    pub fn try_new(name: &str) -> Result<Self, EmbeddingModelError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            Err(EmbeddingModelError::Empty)
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }

    /// Returns the model name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for EmbeddingModel {
    type Error = EmbeddingModelError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        Self::try_new(name)
    }
}

/// A dense vector produced by an embedding model.
#[derive(Clone, Debug, PartialEq)]
pub struct Embedding(Vec<f32>);

impl Embedding {
    /// Creates an embedding from its vector values.
    #[must_use]
    pub fn new(values: Vec<f32>) -> Self {
        Self(values)
    }

    /// Returns the vector values.
    #[must_use]
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// Returns the number of dimensions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the embedding has no dimensions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A passage selected for embedding, carrying its logical identity.
///
/// The logical identity `(file, kind, position)` is stable across lexical
/// index rebuilds because it does not depend on the physical FTS5 rowid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticPassage {
    file: FileId,
    kind: PassageKind,
    position: usize,
    text: String,
}

impl SemanticPassage {
    /// Creates a passage-to-embed with its logical identity and text.
    #[must_use]
    pub fn new(file: FileId, kind: PassageKind, position: usize, text: String) -> Self {
        Self {
            file,
            kind,
            position,
            text,
        }
    }

    /// Returns the owning file's ID.
    #[must_use]
    pub const fn file(&self) -> FileId {
        self.file
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> PassageKind {
        self.kind
    }

    /// Returns the passage's position within its file.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns the passage text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Hashes a collection's stored file set for staleness detection.
///
/// The fingerprint is deterministic over the sorted `(path, content hash)`
/// pairs, so identical file sets hash identically regardless of input order and
/// any file addition, removal, or content change produces a different
/// fingerprint.
#[must_use]
pub fn file_set_fingerprint(files: &[(&Path, &ContentHash)]) -> ContentHash {
    let mut entries = files.to_vec();
    entries.sort_by_key(|(left, right)| (*left, right.as_str().to_owned()));

    let mut combined = Vec::new();
    for (path, hash) in entries {
        combined.extend_from_slice(path.as_os_str().as_encoded_bytes());
        combined.push(b'\n');
        combined.extend_from_slice(hash.as_str().as_bytes());
        combined.push(b'\n');
    }

    ContentHash::from_content(&combined)
}

/// The semantic index status of one collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticIndexStatus {
    file_set_fingerprint: ContentHash,
    model: EmbeddingModel,
    passage_count: usize,
    embedded_at: Timestamp,
}

impl SemanticIndexStatus {
    /// Creates a semantic index status record.
    #[must_use]
    pub fn new(
        file_set_fingerprint: ContentHash,
        model: EmbeddingModel,
        passage_count: usize,
        embedded_at: Timestamp,
    ) -> Self {
        Self {
            file_set_fingerprint,
            model,
            passage_count,
            embedded_at,
        }
    }

    /// Returns the stored file-set fingerprint the vectors were built from.
    #[must_use]
    pub fn file_set_fingerprint(&self) -> &ContentHash {
        &self.file_set_fingerprint
    }

    /// Returns the model the vectors were generated with.
    #[must_use]
    pub fn model(&self) -> &EmbeddingModel {
        &self.model
    }

    /// Returns the number of embedded passages.
    #[must_use]
    pub const fn passage_count(&self) -> usize {
        self.passage_count
    }

    /// Returns when the semantic index was last embedded.
    #[must_use]
    pub const fn embedded_at(&self) -> Timestamp {
        self.embedded_at
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use proptest::prelude::*;
    use rstest::rstest;

    use super::SemanticIndexStatus;
    use super::file_set_fingerprint;
    use super::{Embedding, EmbeddingModel, EmbeddingModelError, SemanticPassage};
    use crate::ContentHash;
    use crate::file_id::FileId;
    use crate::passage::PassageKind;

    /// Covers: REQ-010 — a non-empty model name is accepted.
    #[rstest]
    #[case("all-MiniLM-L6-v2")]
    #[case("  all-MiniLM-L6-v2  ")]
    fn accepts_non_empty_model_names(#[case] name: &str) -> Result<(), EmbeddingModelError> {
        let model = EmbeddingModel::try_new(name)?;

        assert_eq!(model.as_str(), "all-MiniLM-L6-v2");

        Ok(())
    }

    /// Covers: REQ-010 — empty model names are rejected.
    #[rstest]
    #[case("")]
    #[case("   ")]
    fn rejects_empty_model_names(#[case] name: &str) {
        assert_eq!(
            EmbeddingModel::try_new(name),
            Err(EmbeddingModelError::Empty)
        );
        assert_eq!(
            EmbeddingModel::try_from(name),
            Err(EmbeddingModelError::Empty)
        );
    }

    /// Covers: the vector value type exposes its values and length.
    #[test]
    fn embedding_exposes_values_and_length() {
        let embedding = Embedding::new(vec![0.1, 0.2, 0.3]);

        assert_eq!(embedding.len(), 3);
        assert_eq!(embedding.as_slice(), &[0.1, 0.2, 0.3]);
        assert!(!embedding.is_empty());
    }

    /// Covers: an empty embedding reports as empty.
    #[test]
    fn embedding_reports_empty() {
        let embedding = Embedding::new(Vec::new());

        assert!(embedding.is_empty());
        assert_eq!(embedding.len(), 0);
    }

    /// Covers: the passage exposes its logical identity.
    #[test]
    fn semantic_passage_exposes_its_identity() -> Result<(), crate::FileIdError> {
        let file = FileId::try_new(7)?;
        let passage =
            SemanticPassage::new(file, PassageKind::Body, 2, "ownership rules".to_owned());

        assert_eq!(passage.file(), file);
        assert_eq!(passage.kind(), PassageKind::Body);
        assert_eq!(passage.position(), 2);
        assert_eq!(passage.text(), "ownership rules");

        Ok(())
    }

    /// Covers: DES-010 — the fingerprint is deterministic for identical sets.
    #[test]
    fn fingerprint_is_deterministic_for_identical_sets() -> Result<(), crate::ContentHashError> {
        let a = ContentHash::try_from_hex(&"a".repeat(64))?;
        let b = ContentHash::try_from_hex(&"b".repeat(64))?;
        let path_a = PathBuf::from("a.md");
        let path_b = PathBuf::from("b.md");

        let first = file_set_fingerprint(&[(&path_a, &a), (&path_b, &b)]);
        let second = file_set_fingerprint(&[(&path_b, &b), (&path_a, &a)]);

        assert_eq!(first, second);

        Ok(())
    }

    /// Covers: DES-010 — a changed file set changes the fingerprint.
    #[test]
    fn fingerprint_changes_when_the_file_set_changes() -> Result<(), crate::ContentHashError> {
        let a = ContentHash::try_from_hex(&"a".repeat(64))?;
        let b = ContentHash::try_from_hex(&"b".repeat(64))?;
        let path = PathBuf::from("a.md");

        let before = file_set_fingerprint(&[(&path, &a)]);
        let after = file_set_fingerprint(&[(&path, &b)]);

        assert_ne!(before, after);

        Ok(())
    }

    /// Covers: DES-010 — a removed file changes the fingerprint.
    #[test]
    fn fingerprint_changes_when_a_file_is_removed() -> Result<(), crate::ContentHashError> {
        let a = ContentHash::try_from_hex(&"a".repeat(64))?;
        let path = PathBuf::from("a.md");

        let before = file_set_fingerprint(&[(&path, &a)]);
        let after = file_set_fingerprint(&[]);

        assert_ne!(before, after);

        Ok(())
    }

    /// Covers: the status exposes its recorded fields.
    #[test]
    fn semantic_status_exposes_recorded_fields() -> Result<(), Box<dyn std::error::Error>> {
        let model = EmbeddingModel::try_new("all-MiniLM-L6-v2")?;
        let fingerprint = ContentHash::from_content(b"files");
        let status = SemanticIndexStatus::new(
            fingerprint,
            model,
            5,
            crate::Timestamp::from_unix_seconds(0),
        );

        assert_eq!(status.passage_count(), 5);
        assert_eq!(status.model().as_str(), "all-MiniLM-L6-v2");
        assert_eq!(
            status.file_set_fingerprint(),
            &ContentHash::from_content(b"files")
        );
        assert_eq!(status.embedded_at(), crate::Timestamp::from_unix_seconds(0));

        Ok(())
    }

    proptest! {
        /// Covers: DES-010 — fingerprinting is deterministic for arbitrary sets.
        #[test]
        fn fingerprint_is_deterministic_for_arbitrary_sets(
            names in proptest::collection::vec("[a-z]{1,8}\\.md", 0..8),
            hashes in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 16), 0..8),
        ) {
            let min = names.len().min(hashes.len());
            let mut files = Vec::new();
            for (name, bytes) in names.iter().take(min).zip(hashes.iter().take(min)) {
                let path = PathBuf::from(name);
                let hash = ContentHash::from_content(bytes);
                files.push((path, hash));
            }

            let first = file_set_fingerprint(&files.iter().map(|(p, h)| (p.as_path(), h)).collect::<Vec<_>>());
            let mut reversed = files.clone();
            reversed.reverse();
            let second = file_set_fingerprint(&reversed.iter().map(|(p, h)| (p.as_path(), h)).collect::<Vec<_>>());

            prop_assert_eq!(first, second);
        }
    }
}
