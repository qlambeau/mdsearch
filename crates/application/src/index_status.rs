use crate::{IndexStatus, IndexStatusError, LexicalIndexStore};

/// Reads the lexical index status of every collection.
pub struct ReadIndexStatus<S> {
    store: S,
}

impl<S> ReadIndexStatus<S> {
    /// Creates an index-status use case with its store port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }
}

impl<S: LexicalIndexStore> ReadIndexStatus<S> {
    /// Returns the lexical index status of every collection.
    ///
    /// # Errors
    ///
    /// Returns an index-store error when the status cannot be read.
    pub fn execute(&self) -> Result<Vec<IndexStatus>, IndexStatusError> {
        Ok(self.store.status()?)
    }
}

impl<S: crate::IndexInspectionStore> ReadIndexStatus<S> {
    /// Returns a read-only, consistently scoped inspection (REQ-023 FR-022).
    ///
    /// # Errors
    /// Returns an unknown-scope or storage error from the inspection port.
    pub fn inspect(
        &self,
        collection: Option<&kv_domain::CollectionName>,
    ) -> Result<crate::DatabaseIndexInspection, IndexStatusError> {
        Ok(self.store.inspect(collection)?)
    }
}
