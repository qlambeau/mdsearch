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
}
