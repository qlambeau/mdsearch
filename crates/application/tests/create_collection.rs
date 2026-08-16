//! Acceptance tests for the collection-creation application use case.

use std::error::Error;

use kv_application::ClockError;
use kv_application::{Clock, CollectionStore, CollectionStoreError, CreateCollection};
use kv_domain::{CollectionName, Timestamp};

#[derive(Default)]
struct InMemoryStore {
    created_names: Vec<String>,
}

impl CollectionStore for InMemoryStore {
    fn create_collection(
        &mut self,
        name: &CollectionName,
        _created_at: Timestamp,
    ) -> Result<(), CollectionStoreError> {
        self.created_names.push(name.display_name().to_owned());
        Ok(())
    }

    fn list_collections(&self) -> Result<Vec<CollectionName>, CollectionStoreError> {
        self.created_names
            .iter()
            .map(|name| CollectionName::try_from(name.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| CollectionStoreError::Storage(Box::new(error)))
    }
}

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Result<Timestamp, ClockError> {
        Ok(Timestamp::from_unix_seconds(1_700_000_000))
    }
}

/// Covers: FR-008 — valid creation persists one empty collection.
#[test]
fn creates_a_collection_through_the_application_use_case() -> Result<(), Box<dyn Error>> {
    let name = CollectionName::try_from("Notes")?;
    let store = InMemoryStore::default();
    let mut use_case = CreateCollection::new(store, FixedClock);

    let created_name = use_case.execute(name)?;

    assert_eq!(created_name.display_name(), "Notes");
    assert_eq!(use_case.store().created_names, ["Notes"]);

    Ok(())
}
