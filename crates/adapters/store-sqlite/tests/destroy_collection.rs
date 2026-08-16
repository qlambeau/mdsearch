//! Integration tests for the `SQLite` collection destroy path.

use std::error::Error;

use kv_application::{CollectionStore, CollectionStoreError};
use kv_domain::{CollectionName, Timestamp};
use tempfile::tempdir;

use kv_store_sqlite::SqliteCollectionStore;

/// Covers: FR-004 — destruction matches case-insensitively and returns the retained spelling.
#[test]
fn destroys_a_collection_case_insensitively() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database_path)?;

    store.create_collection(
        &CollectionName::try_from("Notes")?,
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let destroyed = store.destroy_collection(&CollectionName::try_from("notes")?)?;

    assert_eq!(destroyed.display_name(), "Notes");
    assert!(store.list_collections()?.is_empty());

    Ok(())
}

/// Covers: FR-006 — destroying an absent collection reports not found.
#[test]
fn reports_not_found_for_an_absent_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database_path)?;

    let error = store
        .destroy_collection(&CollectionName::try_from("Missing")?)
        .err()
        .ok_or_else(|| std::io::Error::other("an absent collection should fail to destroy"))?;

    assert!(matches!(error, CollectionStoreError::CollectionNotFound));

    Ok(())
}

/// Covers: FR-007 — a missing database is reported without creating a file.
#[test]
fn reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");

    let error = SqliteCollectionStore::open_existing(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing database should fail to open"))?;

    assert!(matches!(error, CollectionStoreError::DatabaseNotFound));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-009 — destroying one collection leaves others intact.
#[test]
fn leaves_other_collections_intact() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database_path)?;

    for raw_name in ["Notes", "Archive"] {
        store.create_collection(
            &CollectionName::try_from(raw_name)?,
            Timestamp::from_unix_seconds(1_700_000_000),
        )?;
    }

    store.destroy_collection(&CollectionName::try_from("Notes")?)?;

    let names = store
        .list_collections()?
        .into_iter()
        .map(|name| name.display_name().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(names, ["Archive"]);

    Ok(())
}
