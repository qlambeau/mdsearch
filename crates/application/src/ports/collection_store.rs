use kv_domain::{CollectionName, Timestamp};

use crate::CollectionStoreError;

/// Persists empty collections for the collection-management use cases.
pub trait CollectionStore {
    /// Creates one empty collection or reports why it could not be created.
    ///
    /// # Errors
    ///
    /// Returns a duplicate, database, or storage error when persistence cannot
    /// complete.
    fn create_collection(
        &mut self,
        name: &CollectionName,
        created_at: Timestamp,
    ) -> Result<(), CollectionStoreError>;

    /// Returns all collections in case-insensitive alphabetical order.
    ///
    /// # Errors
    ///
    /// Returns a database or storage error when the collections cannot be read.
    fn list_collections(&self) -> Result<Vec<CollectionName>, CollectionStoreError>;

    /// Destroys the collection matching the supplied name, returning the
    /// retained spelling of the destroyed collection.
    ///
    /// # Errors
    ///
    /// Returns a not-found, database, or storage error when destruction cannot
    /// complete.
    fn destroy_collection(
        &mut self,
        name: &CollectionName,
    ) -> Result<CollectionName, CollectionStoreError>;
}
