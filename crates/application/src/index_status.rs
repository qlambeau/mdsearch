use crate::{IndexStatus, IndexStatusError, LexicalIndexStore};

/// Reads the lexical index status of every collection.
pub struct ReadIndexStatus<S> {
    store: S,
}

impl<S> ReadIndexStatus<S>
where
    S: LexicalIndexStore,
{
    /// Creates an index-status use case with its store port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns the lexical index status of every collection.
    ///
    /// # Errors
    ///
    /// Returns an index-store error when the status cannot be read.
    pub fn execute(&self) -> Result<Vec<IndexStatus>, IndexStatusError> {
        Ok(self.store.status()?)
    }
}
