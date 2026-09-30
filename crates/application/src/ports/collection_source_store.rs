use kv_domain::{CollectionName, CollectionSource, Timestamp};

use crate::CollectionStoreError;

/// A collection name and its registered filesystem sources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionSourceSummary {
    /// The retained collection name.
    pub name: CollectionName,
    /// The registered canonical source paths.
    pub sources: Vec<CollectionSource>,
    /// Whether semantic indexing is configured for this collection.
    pub semantic_enabled: bool,
}

/// Persists collection source configuration.
pub trait CollectionSourceStore {
    /// Creates a collection and its initial sources atomically.
    ///
    /// # Errors
    ///
    /// Returns a duplicate, database, or storage error if creation fails.
    fn create_collection_with_sources(
        &mut self,
        name: &CollectionName,
        created_at: Timestamp,
        sources: &[CollectionSource],
    ) -> Result<(), CollectionStoreError>;

    /// Replaces a collection's registered source set atomically.
    ///
    /// # Errors
    ///
    /// Returns not-found, database, or storage error if replacement fails.
    fn replace_collection_sources(
        &mut self,
        name: &CollectionName,
        sources: &[CollectionSource],
    ) -> Result<(), CollectionStoreError>;

    /// Returns the registered sources for a collection.
    ///
    /// # Errors
    ///
    /// Returns not-found, database, or storage error if the sources cannot be read.
    fn collection_sources(
        &self,
        name: &CollectionName,
    ) -> Result<Vec<CollectionSource>, CollectionStoreError>;

    /// Returns every collection and its source list in stable name order.
    ///
    /// # Errors
    ///
    /// Returns a database or storage error if the summaries cannot be read.
    fn list_collections_with_sources(
        &self,
    ) -> Result<Vec<CollectionSourceSummary>, CollectionStoreError>;
}

#[cfg(test)]
mod tests {
    use super::fake::InMemoryCollectionSourceStore;
    use super::*;

    /// Covers: REQ-021 FR-001, FR-002 — the in-memory port double persists and replaces sources.
    #[test]
    fn source_store_fake_persists_and_replaces_sources() -> Result<(), Box<dyn std::error::Error>> {
        let name = CollectionName::try_from("Notes")?;
        let initial = CollectionSource::new("/vault".into(), kv_domain::SourceKind::Directory);
        let replacement =
            CollectionSource::new("/archive".into(), kv_domain::SourceKind::Directory);
        let mut store = InMemoryCollectionSourceStore::default();

        store.create_collection_with_sources(&name, Timestamp::from_unix_seconds(1), &[initial])?;
        store.replace_collection_sources(&name, std::slice::from_ref(&replacement))?;

        assert_eq!(store.collection_sources(&name)?, vec![replacement]);
        Ok(())
    }
}

/// In-memory source store for application tests.
#[cfg(test)]
pub mod fake {
    use std::collections::BTreeMap;

    use super::*;

    /// Stores source configurations in memory.
    #[derive(Default, Clone)]
    pub struct InMemoryCollectionSourceStore {
        values: BTreeMap<String, CollectionSourceSummary>,
    }

    impl CollectionSourceStore for InMemoryCollectionSourceStore {
        fn create_collection_with_sources(
            &mut self,
            name: &CollectionName,
            _created_at: Timestamp,
            sources: &[CollectionSource],
        ) -> Result<(), CollectionStoreError> {
            if self.values.contains_key(name.name_key()) {
                return Err(CollectionStoreError::Duplicate);
            }
            self.values.insert(
                name.name_key().to_owned(),
                CollectionSourceSummary {
                    name: name.clone(),
                    sources: sources.to_vec(),
                    semantic_enabled: false,
                },
            );
            Ok(())
        }

        fn replace_collection_sources(
            &mut self,
            name: &CollectionName,
            sources: &[CollectionSource],
        ) -> Result<(), CollectionStoreError> {
            let summary = self
                .values
                .get_mut(name.name_key())
                .ok_or(CollectionStoreError::CollectionNotFound)?;
            summary.sources = sources.to_vec();
            Ok(())
        }

        fn collection_sources(
            &self,
            name: &CollectionName,
        ) -> Result<Vec<CollectionSource>, CollectionStoreError> {
            self.values
                .get(name.name_key())
                .map(|summary| summary.sources.clone())
                .ok_or(CollectionStoreError::CollectionNotFound)
        }

        fn list_collections_with_sources(
            &self,
        ) -> Result<Vec<CollectionSourceSummary>, CollectionStoreError> {
            Ok(self.values.values().cloned().collect())
        }
    }
}
