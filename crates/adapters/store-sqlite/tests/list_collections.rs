//! Integration tests for the read-only `SQLite` collection listing path.

use std::error::Error;

use kv_application::{CollectionStore, CollectionStoreError};
use kv_domain::{CollectionName, Timestamp};
use tempfile::tempdir;

use kv_store_sqlite::SqliteCollectionStore;

/// Covers: FR-006 and FR-008 — a missing database is reported without creating a file.
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

/// Covers: FR-004 and FR-005 — names are returned in case-insensitive alphabetical order.
#[test]
fn lists_collections_in_case_insensitive_alphabetical_order() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database_path)?;

    for raw_name in ["banana", "Apple", "cherry"] {
        store.create_collection(
            &CollectionName::try_from(raw_name)?,
            Timestamp::from_unix_seconds(1_700_000_000),
        )?;
    }

    let names = store
        .list_collections()?
        .into_iter()
        .map(|name| name.display_name().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(names, ["Apple", "banana", "cherry"]);

    Ok(())
}

/// Covers: FR-005 — an existing empty database lists no names.
#[test]
fn lists_no_collections_for_an_empty_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let store = SqliteCollectionStore::open(&database_path)?;

    let names = store.list_collections()?;

    assert!(names.is_empty());

    Ok(())
}

/// Covers: FR-007 — an existing path that cannot be opened fails semantically.
#[test]
fn reports_an_unopenable_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("a_directory_as_database");
    std::fs::create_dir(&database_path)?;

    let error = SqliteCollectionStore::open_existing(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a directory should not open as a database"))?;

    assert!(matches!(
        error,
        CollectionStoreError::DatabaseUnavailable(_)
    ));

    Ok(())
}
