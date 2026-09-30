//! Integration tests for the `SQLite` collection store.

use std::error::Error;

use kv_application::CollectionStore;
use kv_domain::{CollectionName, Timestamp};
use rusqlite::Connection;
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

/// Covers: REQ-022 FR-001/FR-006 — new collections default to semantic-off in schema v9.
#[test]
fn new_collection_has_semantic_disabled_policy_in_schema_nine() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let name = CollectionName::try_from("Notes")?;
    let mut store = SqliteCollectionStore::open(&database_path)?;
    store.create_collection(&name, Timestamp::from_unix_seconds(1_700_000_000))?;

    let connection = Connection::open(&database_path)?;
    let version: i64 =
        connection.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })?;
    let enabled: i64 = connection.query_row(
        "SELECT semantic_enabled FROM collections WHERE display_name = 'Notes'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(version, 9);
    assert_eq!(enabled, 0);
    Ok(())
}

/// Covers: REQ-022 FR-006 — v8 migration enables only collections with semantic state.
#[test]
fn schema_eight_migration_backfills_semantic_policy_from_existing_index_state()
-> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("legacy.db");
    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TABLE schema_version(version INTEGER NOT NULL);
         INSERT INTO schema_version VALUES (8);
         CREATE TABLE collections(
             collection_id INTEGER PRIMARY KEY, display_name TEXT NOT NULL,
             name_key TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL
         );
         INSERT INTO collections VALUES (1, 'Enabled', 'enabled', 1);
         INSERT INTO collections VALUES (2, 'Disabled', 'disabled', 1);
         CREATE TABLE semantic_index_state(
             collection_id INTEGER PRIMARY KEY, file_set_fingerprint TEXT NOT NULL,
             model TEXT NOT NULL, dimension INTEGER, passage_count INTEGER NOT NULL,
             embedded_at INTEGER NOT NULL
         );
         INSERT INTO semantic_index_state VALUES (1, 'hash', 'model', 384, 1, 1);",
    )?;
    drop(connection);

    SqliteCollectionStore::open(&database_path)?;
    let connection = Connection::open(&database_path)?;
    let enabled: i64 = connection.query_row(
        "SELECT semantic_enabled FROM collections WHERE name_key = 'enabled'",
        [],
        |row| row.get(0),
    )?;
    let disabled: i64 = connection.query_row(
        "SELECT semantic_enabled FROM collections WHERE name_key = 'disabled'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(enabled, 1);
    assert_eq!(disabled, 0);
    Ok(())
}
