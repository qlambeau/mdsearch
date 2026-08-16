use kv_domain::CollectionName;

use crate::{CollectionStore, DestroyCollectionError};

/// Orchestrates destruction of one named collection.
pub struct DestroyCollection<S> {
    store: S,
}

impl<S: CollectionStore> DestroyCollection<S> {
    /// Creates a destroy-collection use case with its persistence port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Destroys the collection matching the supplied name.
    ///
    /// # Errors
    ///
    /// Returns a collection-store error when the destruction cannot complete.
    pub fn execute(
        &mut self,
        name: &CollectionName,
    ) -> Result<CollectionName, DestroyCollectionError> {
        Ok(self.store.destroy_collection(name)?)
    }
}
