//! Integration tests for the `SQLite` collection store.

use std::error::Error;

use kv_application::CollectionStore;
use kv_domain::{CollectionName, Timestamp};
use tempfile::tempdir;

use kv_store_sqlite::SqliteCollectionStore;

/// Covers: FR-007 and FR-008 — initialize the database and create one empty collection.
#[test]
fn initializes_database_and_creates_an_empty_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");
    let name = CollectionName::try_from("Notes")?;
    let mut store = SqliteCollectionStore::open(&database_path)?;

    store.create_collection(&name, Timestamp::from_unix_seconds(1_700_000_000))?;

    assert!(database_path.exists());

    Ok(())
}
