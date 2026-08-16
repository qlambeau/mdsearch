//! Acceptance tests for the list-collections application use case.

use std::error::Error;

use kv_application::{CollectionStore, CollectionStoreError, ListCollections};
use kv_domain::{CollectionName, Timestamp};

#[derive(Default)]
struct InMemoryStore {
    names: Vec<String>,
}

impl CollectionStore for InMemoryStore {
    fn create_collection(
        &mut self,
        name: &CollectionName,
        _created_at: Timestamp,
    ) -> Result<(), CollectionStoreError> {
        self.names.push(name.display_name().to_owned());
        Ok(())
    }

    fn list_collections(&self) -> Result<Vec<CollectionName>, CollectionStoreError> {
        self.names
            .iter()
            .map(|name| CollectionName::try_from(name.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| CollectionStoreError::Storage(Box::new(error)))
    }
}

/// Covers: FR-004 — the use case passes through stored collection names.
#[test]
fn lists_collections_through_the_application_use_case() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryStore::default();
    store.create_collection(
        &CollectionName::try_from("Notes")?,
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;
    let use_case = ListCollections::new(store);

    let collections = use_case.execute()?;

    let names = collections
        .iter()
        .map(CollectionName::display_name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["Notes"]);

    Ok(())
}

/// Covers: FR-005 — an empty store lists no collections.
#[test]
fn lists_no_collections_when_the_store_is_empty() -> Result<(), Box<dyn Error>> {
    let use_case = ListCollections::new(InMemoryStore::default());

    let collections = use_case.execute()?;

    assert!(collections.is_empty());

    Ok(())
}
