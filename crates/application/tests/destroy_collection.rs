//! Acceptance tests for the destroy-collection application use case.

use std::error::Error;

use kv_application::{CollectionStore, CollectionStoreError, DestroyCollection};
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

    fn destroy_collection(
        &mut self,
        name: &CollectionName,
    ) -> Result<CollectionName, CollectionStoreError> {
        let index = self
            .names
            .iter()
            .position(|stored| stored.eq_ignore_ascii_case(name.display_name()))
            .ok_or(CollectionStoreError::CollectionNotFound)?;

        let display_name = self.names.remove(index);
        CollectionName::try_from(display_name.as_str())
            .map_err(|error| CollectionStoreError::Storage(Box::new(error)))
    }
}

/// Covers: FR-004 and FR-008 — destroying returns the retained spelling.
#[test]
fn destroys_a_collection_and_returns_the_retained_spelling() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryStore::default();
    store.create_collection(
        &CollectionName::try_from("Notes")?,
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;
    let mut use_case = DestroyCollection::new(store);

    let destroyed = use_case.execute(&CollectionName::try_from("notes")?)?;

    assert_eq!(destroyed.display_name(), "Notes");

    Ok(())
}

/// Covers: FR-006 — destroying an absent collection reports not found.
#[test]
fn reports_not_found_when_the_collection_is_absent() -> Result<(), Box<dyn Error>> {
    let mut use_case = DestroyCollection::new(InMemoryStore::default());

    let error = use_case
        .execute(&CollectionName::try_from("Missing")?)
        .err()
        .ok_or_else(|| std::io::Error::other("an absent collection should fail to destroy"))?;

    assert!(matches!(
        error,
        kv_application::DestroyCollectionError::Store(CollectionStoreError::CollectionNotFound)
    ));

    Ok(())
}
