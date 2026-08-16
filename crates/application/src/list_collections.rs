use kv_domain::CollectionName;

use crate::{CollectionStore, ListCollectionsError};

/// Orchestrates listing all collections in the selected database.
pub struct ListCollections<S> {
    store: S,
}

impl<S: CollectionStore> ListCollections<S> {
    /// Creates a list-collections use case with its persistence port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns all collections in case-insensitive alphabetical order.
    ///
    /// # Errors
    ///
    /// Returns a collection-store error when the listing cannot complete.
    pub fn execute(&self) -> Result<Vec<CollectionName>, ListCollectionsError> {
        Ok(self.store.list_collections()?)
    }
}
