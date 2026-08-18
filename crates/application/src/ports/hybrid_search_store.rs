use std::path::{Path, PathBuf};

use kv_domain::{CollectionName, Embedding, EmbeddingModel, PassageKey, PassageKind};

use crate::HybridSearchStoreError;
use crate::Position;
use crate::SearchScope;

/// One candidate passage retrieved by a hybrid search leg.
#[derive(Clone, Debug)]
pub struct HybridCandidate {
    key: PassageKey,
    collection: CollectionName,
    path: PathBuf,
    kind: PassageKind,
    text: String,
    score: f64,
    position: Position,
}

impl HybridCandidate {
    /// Creates a candidate from its logical identity, provenance, and score.
    #[must_use]
    pub fn new(
        key: PassageKey,
        collection: CollectionName,
        path: PathBuf,
        kind: PassageKind,
        text: String,
        score: f64,
        position: Position,
    ) -> Self {
        Self {
            key,
            collection,
            path,
            kind,
            text,
            score,
            position,
        }
    }

    /// Returns the passage's logical fusion key.
    #[must_use]
    pub const fn key(&self) -> PassageKey {
        self.key
    }

    /// Returns the collection the passage belongs to.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }

    /// Returns the file path of the passage.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> PassageKind {
        self.kind
    }

    /// Returns the passage text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the per-leg score (negated BM25 or cosine similarity, higher is
    /// better).
    #[must_use]
    pub const fn score(&self) -> f64 {
        self.score
    }

    /// Returns the passage's position in its file.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

/// The oversampled lexical and semantic candidate lists of one hybrid search.
#[derive(Clone, Debug, Default)]
pub struct HybridCandidates {
    lexical: Vec<HybridCandidate>,
    semantic: Vec<HybridCandidate>,
}

impl HybridCandidates {
    /// Creates a candidate set from its two ranked lists.
    #[must_use]
    pub const fn new(lexical: Vec<HybridCandidate>, semantic: Vec<HybridCandidate>) -> Self {
        Self { lexical, semantic }
    }

    /// Returns the ranked lexical candidates.
    #[must_use]
    pub fn lexical(&self) -> &[HybridCandidate] {
        &self.lexical
    }

    /// Returns the ranked semantic candidates.
    #[must_use]
    pub fn semantic(&self) -> &[HybridCandidate] {
        &self.semantic
    }
}

/// Retrieves hybrid search candidates and the models they depend on.
pub trait HybridSearchStore {
    /// Returns the recorded global embedding model, if any.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be read.
    fn global_model(&self) -> Result<Option<EmbeddingModel>, HybridSearchStoreError>;

    /// Returns the recorded global re-ranker model, if any.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be read.
    fn reranker_model(&self) -> Result<Option<kv_domain::RerankerModel>, HybridSearchStoreError>;

    /// Retrieves up to `pool` ranked lexical and semantic candidates within
    /// `scope`.
    ///
    /// The lexical leg runs the `fts5_query` match; the semantic leg runs a
    /// `knn_match` against the stored vectors using `query_embedding`, which is
    /// `None` when no global embedding model is recorded (in which case no
    /// collection has a semantic index and the semantic leg is skipped). A
    /// collection with a built lexical index but no semantic index contributes
    /// to the lexical leg only. For [`SearchScope::All`], collections without a
    /// built lexical index are skipped; for [`SearchScope::Collection`], an
    /// unknown collection or one without a built index is an error. If any
    /// in-scope collection's semantic index is stale, the call fails.
    ///
    /// # Errors
    ///
    /// Returns a not-found, not-built, stale-semantic-index, or storage error
    /// when the retrieval cannot complete.
    fn candidates(
        &self,
        fts5_query: &str,
        query_embedding: Option<&Embedding>,
        scope: SearchScope<'_>,
        pool: usize,
    ) -> Result<HybridCandidates, HybridSearchStoreError>;
}
