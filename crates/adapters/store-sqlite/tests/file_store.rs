//! Integration tests for the `SQLite` file store and schema migration.

use std::error::Error;
use std::path::PathBuf;

use kv_application::{CollectionStore, FileRecord, FileStore, FileStoreError};
use kv_domain::{CollectionName, Timestamp};
use rusqlite::Connection;
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore};

fn name() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

/// Covers: FR-010 and the schema version 2 migration.
#[test]
fn open_creates_the_files_table_at_version_two() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    SqliteCollectionStore::open(&database_path)?;

    let connection = Connection::open(&database_path)?;
    let version: i64 =
        connection.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })?;
    let files_table: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'files'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(version, 2);
    assert_eq!(files_table, 1);

    Ok(())
}

/// Covers: FR-009 — re-adding a path replaces content without duplicating.
#[test]
fn upsert_replaces_content_without_duplicating() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path.clone(), b"first".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path.clone(), b"second content".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;

    let connection = Connection::open(&database_path)?;
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))?;
    let content: Vec<u8> =
        connection.query_row("SELECT content FROM files LIMIT 1", [], |row| row.get(0))?;

    assert_eq!(count, 1);
    assert_eq!(content, b"second content".to_vec());

    Ok(())
}

/// Covers: FR-014 — re-adding retains the stable file ID and creation time.
#[test]
fn upsert_retains_file_id_and_creation_time() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path.clone(), b"first".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let connection = Connection::open(&database_path)?;
    let file_id_before: i64 =
        connection.query_row("SELECT file_id FROM files LIMIT 1", [], |row| row.get(0))?;
    let created_before: i64 =
        connection.query_row("SELECT created_at FROM files LIMIT 1", [], |row| row.get(0))?;

    store.upsert_files(
        &collection,
        &[FileRecord::new(path, b"second".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;

    let connection = Connection::open(&database_path)?;
    let file_id_after: i64 =
        connection.query_row("SELECT file_id FROM files LIMIT 1", [], |row| row.get(0))?;
    let created_after: i64 =
        connection.query_row("SELECT created_at FROM files LIMIT 1", [], |row| row.get(0))?;

    assert_eq!(file_id_before, file_id_after);
    assert_eq!(created_before, created_after);

    Ok(())
}

/// Covers: FR-005 — upserting into a missing collection reports not found.
#[test]
fn reports_collection_not_found() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    SqliteCollectionStore::open(&database_path)?;

    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let record = FileRecord::new(PathBuf::from("a.md"), b"content".to_vec());

    let error = store
        .upsert_files(
            &name()?,
            &[record],
            Timestamp::from_unix_seconds(1_700_000_000),
        )
        .err()
        .ok_or_else(|| std::io::Error::other("a missing collection should fail"))?;

    assert!(matches!(error, FileStoreError::CollectionNotFound));

    Ok(())
}

/// Covers: the schema version 1 to 2 migration.
#[test]
fn migrates_a_version_one_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");

    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TABLE schema_version (version INTEGER NOT NULL);
         INSERT INTO schema_version(version) VALUES (1);
         CREATE TABLE collections (
             collection_id INTEGER PRIMARY KEY,
             display_name TEXT NOT NULL,
             name_key TEXT NOT NULL UNIQUE,
             created_at INTEGER NOT NULL
         );",
    )?;
    drop(connection);

    SqliteFileStore::open_for_ingestion(&database_path)?;

    let connection = Connection::open(&database_path)?;
    let version: i64 =
        connection.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })?;

    assert_eq!(version, 2);

    Ok(())
}

/// Covers: FR-006 — ingestion of a missing database fails without creation.
#[test]
fn reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");

    let error = SqliteFileStore::open_for_ingestion(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(matches!(
        error,
        kv_application::CollectionStoreError::DatabaseNotFound
    ));
    assert!(!database_path.exists());

    Ok(())
}
