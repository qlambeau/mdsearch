use kv_domain::{CollectionName, Timestamp};

use crate::{Clock, CollectionStore, CreateCollectionError};

/// Orchestrates creation of one empty collection.
pub struct CreateCollection<S, C> {
    store: S,
    clock: C,
}

impl<S, C> CreateCollection<S, C>
where
    S: CollectionStore,
    C: Clock,
{
    /// Creates a collection use case with its persistence and clock ports.
    #[must_use]
    pub const fn new(store: S, clock: C) -> Self {
        Self { store, clock }
    }

    /// Persists the supplied validated collection name.
    ///
    /// # Errors
    ///
    /// Returns a clock or collection-store error when creation cannot complete.
    pub fn execute(
        &mut self,
        name: CollectionName,
    ) -> Result<CollectionName, CreateCollectionError> {
        let created_at: Timestamp = self.clock.now()?;

        self.store.create_collection(&name, created_at)?;

        Ok(name)
    }

    /// Returns the underlying store for composition and integration testing.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}
