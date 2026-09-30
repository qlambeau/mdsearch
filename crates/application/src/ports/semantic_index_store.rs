use std::path::{Path, PathBuf};

use kv_domain::{
    CollectionName, ContentHash, Embedding, EmbeddingModel, RerankerModel, SemanticIndexStatus,
    SemanticPassage, Timestamp,
};

use crate::{FileRecord, SemanticIndexStoreError};

/// A passage embedding whose file identifier is resolved inside the commit transaction.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSemanticPassage {
    path: PathBuf,
    kind: kv_domain::PassageKind,
    position: usize,
    embedding: Embedding,
}

impl PreparedSemanticPassage {
    /// Creates a staged vector for the passage at `path`.
    #[must_use]
    pub const fn new(
        path: PathBuf,
        kind: kv_domain::PassageKind,
        position: usize,
        embedding: Embedding,
    ) -> Self {
        Self {
            path,
            kind,
            position,
            embedding,
        }
    }

    /// Returns the source file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> kv_domain::PassageKind {
        self.kind
    }

    /// Returns the passage position within its file.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Returns the staged vector.
    #[must_use]
    pub const fn embedding(&self) -> &Embedding {
        &self.embedding
    }
}

/// File and semantic-index changes to commit for one collection update.
#[derive(Clone, Debug)]
pub struct CollectionIndexUpdate {
    /// Collection being reconciled.
    pub collection: CollectionName,
    /// Complete current source files to upsert.
    pub upsert: Vec<FileRecord>,
    /// Stored paths that no longer exist in registered sources.
    pub delete: Vec<PathBuf>,
    /// Embeddings staged before the write transaction for enabled collections.
    pub semantic: Vec<PreparedSemanticPassage>,
    /// Global model that produced the staged vectors.
    pub model: EmbeddingModel,
}

/// Prepared embeddings for one collection in a global model rebuild.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedSemanticCollection {
    /// Collection whose vectors are being replaced.
    pub collection: CollectionName,
    /// Passage identities and vectors prepared before storage mutation.
    pub embeddings: Vec<(SemanticPassage, Embedding)>,
}

/// A collection eligible for semantic indexing, with its embed prerequisites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbedTarget {
    collection: CollectionName,
    has_files: bool,
    lexical_built: bool,
}

impl EmbedTarget {
    /// Creates an embed target describing a collection's eligibility.
    #[must_use]
    pub const fn new(collection: CollectionName, has_files: bool, lexical_built: bool) -> Self {
        Self {
            collection,
            has_files,
            lexical_built,
        }
    }

    /// Returns the target collection.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }

    /// Returns whether the collection has at least one stored file.
    #[must_use]
    pub const fn has_files(&self) -> bool {
        self.has_files
    }

    /// Returns whether the collection's lexical index has been built.
    #[must_use]
    pub const fn lexical_built(&self) -> bool {
        self.lexical_built
    }
}

/// Reads and writes the semantic (vector) index and its global model.
pub trait SemanticIndexStore {
    /// Returns whether semantic indexing is enabled for `collection`.
    ///
    /// # Errors
    ///
    /// Returns not-found when the collection does not exist, or storage errors.
    fn semantic_enabled(
        &self,
        collection: &CollectionName,
    ) -> Result<bool, SemanticIndexStoreError>;

    /// Persists whether semantic indexing is enabled for `collection`.
    ///
    /// # Errors
    ///
    /// Returns not-found when the collection does not exist, or storage errors.
    fn set_semantic_enabled(
        &mut self,
        collection: &CollectionName,
        enabled: bool,
    ) -> Result<(), SemanticIndexStoreError>;

    /// Atomically commits file, lexical, graph, and semantic state for one collection.
    ///
    /// # Errors
    ///
    /// Returns not-found or storage failure; errors roll back the complete collection update.
    fn reconcile_collection(
        &mut self,
        update: &CollectionIndexUpdate,
        at: Timestamp,
    ) -> Result<(), SemanticIndexStoreError>;

    /// Returns every collection's embed eligibility.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the status cannot be read.
    fn targets(&self) -> Result<Vec<EmbedTarget>, SemanticIndexStoreError>;

    /// Resolves one collection's embed eligibility by name.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the status cannot be read.
    fn resolve(&self, collection: &CollectionName) -> Result<EmbedTarget, SemanticIndexStoreError>;

    /// Returns the recorded global embedding model, if any.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be read.
    fn global_model(&self) -> Result<Option<EmbeddingModel>, SemanticIndexStoreError>;

    /// Records the global embedding model.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be written.
    fn set_global_model(&mut self, model: &EmbeddingModel) -> Result<(), SemanticIndexStoreError>;

    /// Returns the recorded global re-ranker model, if any.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be read.
    fn reranker_model(&self) -> Result<Option<RerankerModel>, SemanticIndexStoreError>;

    /// Records the global re-ranker model.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the setting cannot be written.
    fn set_reranker_model(&mut self, model: &RerankerModel) -> Result<(), SemanticIndexStoreError>;

    /// Ensures the shared vector table exists at `dimension`.
    ///
    /// When the recorded active dimension differs from `dimension` (or no
    /// table exists yet), the `embeddings` table is recreated at `dimension`
    /// and the `embedding_dimension` setting is updated, transactionally.
    /// This is a no-op when the table already exists at `dimension`.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the table cannot be recreated or the
    /// setting cannot be written.
    fn ensure_dimension(&mut self, dimension: usize) -> Result<(), SemanticIndexStoreError>;

    /// Returns the semantic index status of one collection, if embedded.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the state cannot be read.
    fn status(
        &self,
        collection: &CollectionName,
    ) -> Result<Option<SemanticIndexStatus>, SemanticIndexStoreError>;

    /// Returns the collections that have an existing semantic index.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the state cannot be read.
    fn embedded_collections(&self) -> Result<Vec<CollectionName>, SemanticIndexStoreError>;

    /// Returns the current stored file-set fingerprint for a collection.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the files cannot be read.
    fn file_set_fingerprint(
        &self,
        collection: &CollectionName,
    ) -> Result<ContentHash, SemanticIndexStoreError>;

    /// Returns the lexical passages of a collection ready to be embedded.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the passages cannot be read.
    fn passages(
        &self,
        collection: &CollectionName,
    ) -> Result<Vec<SemanticPassage>, SemanticIndexStoreError>;

    /// Atomically replaces a collection's semantic index.
    ///
    /// Deletes the collection's existing vectors, inserts `embeddings` keyed to
    /// their logical passage identity, and records the collection's semantic
    /// state, all in one transaction. Returns the number of passages embedded.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the rebuild cannot complete.
    fn rebuild(
        &mut self,
        collection: &CollectionName,
        model: &EmbeddingModel,
        embedded_at: Timestamp,
        embeddings: &[(SemanticPassage, Embedding)],
    ) -> Result<usize, SemanticIndexStoreError>;

    /// Atomically switches the global embedding model and replaces all supplied indexes.
    ///
    /// # Errors
    ///
    /// Returns a storage error if any setting, dimension, vector, or state write fails.
    fn rebuild_all(
        &mut self,
        model: &EmbeddingModel,
        reranker: Option<&RerankerModel>,
        embedded_at: Timestamp,
        collections: &[PreparedSemanticCollection],
    ) -> Result<(), SemanticIndexStoreError>;
}
