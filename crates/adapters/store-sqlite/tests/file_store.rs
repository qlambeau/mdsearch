//! Integration tests for the `SQLite` file store and schema migration.

use std::error::Error;
use std::path::PathBuf;

use kv_application::{
    CollectionStore, FileRecord, FileStore, FileStoreError, IndexState, LexicalIndexStore,
};
use kv_domain::{CollectionName, Timestamp};
use rusqlite::Connection;
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore, SqliteLexicalIndexStore};

fn name() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

/// Covers: FR-001 and the schema version 3 migration.
#[test]
fn open_creates_the_index_tables_at_version_three() -> Result<(), Box<dyn Error>> {
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
    let passages_table: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'passages'",
        [],
        |row| row.get(0),
    )?;
    let mapping_table: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'passage_files'",
        [],
        |row| row.get(0),
    )?;
    let state_table: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'lexical_index_state'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(version, 3);
    assert_eq!(files_table, 1);
    assert_eq!(passages_table, 1);
    assert_eq!(mapping_table, 1);
    assert_eq!(state_table, 1);

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

    assert_eq!(version, 3);

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

/// Covers: `list_files` returns stored paths and hashes.
#[test]
fn lists_stored_files_with_hashes() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path.clone(), b"content".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let stored = store.list_files(&collection)?;

    assert_eq!(stored.len(), 1);
    let single = stored
        .first()
        .ok_or_else(|| std::io::Error::other("expected one stored file"))?;
    assert_eq!(single.path(), path.as_path());
    assert_eq!(
        single.content_hash().as_str(),
        kv_domain::ContentHash::from_content(b"content").as_str()
    );

    Ok(())
}

/// Covers: `reconcile` upserts and deletes atomically.
#[test]
fn reconciles_upserts_and_deletes() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let kept = directory.path().join("kept.md");
    let removed = directory.path().join("removed.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(removed.clone(), b"remove".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    store.reconcile(
        &collection,
        &[FileRecord::new(kept.clone(), b"keep".to_vec())],
        std::slice::from_ref(&removed),
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;

    let stored = store.list_files(&collection)?;
    assert_eq!(stored.len(), 1);
    let single = stored
        .first()
        .ok_or_else(|| std::io::Error::other("expected one stored file"))?;
    assert_eq!(single.path(), kept.as_path());

    Ok(())
}

/// Covers: FR-001 — reconcile rebuilds the index and status reports it built.
#[test]
fn reconcile_builds_the_index_and_status_reports_it_built() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let content = "---\ntitle: T\ntags: [x]\n---\n\none\n\ntwo";
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path, content.as_bytes().to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let outcome = store.reconcile(
        &collection,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;
    assert_eq!(outcome.malformed_frontmatter(), 0);

    let status_store = SqliteLexicalIndexStore::open(&database_path)?;
    let statuses = status_store.status()?;
    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;

    assert_eq!(status.state(), IndexState::Built);
    assert_eq!(status.file_count(), 1);
    assert_eq!(status.passage_count(), 4);
    assert_eq!(
        status.built_at(),
        Some(Timestamp::from_unix_seconds(1_700_000_001))
    );

    Ok(())
}

/// Covers: FR-008 — a rebuild replaces stale passages.
#[test]
fn reconcile_removes_stale_passages() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(
            path.clone(),
            b"one\n\ntwo\n\nthree".to_vec(),
        )],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;
    store.reconcile(
        &collection,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;

    let connection = Connection::open(&database_path)?;
    let before: i64 =
        connection.query_row("SELECT COUNT(*) FROM passage_files", [], |row| row.get(0))?;
    assert_eq!(before, 3);

    drop(connection);

    store.reconcile(
        &collection,
        &[FileRecord::new(path, b"only".to_vec())],
        &[],
        Timestamp::from_unix_seconds(1_700_000_002),
    )?;

    let connection = Connection::open(&database_path)?;
    let after: i64 =
        connection.query_row("SELECT COUNT(*) FROM passage_files", [], |row| row.get(0))?;
    assert_eq!(after, 1);

    Ok(())
}

/// Covers: FR-006 — malformed frontmatter is counted and indexed body-only.
#[test]
fn reconcile_reports_malformed_frontmatter() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let content = "---\ntitle: \"unterminated\n: bad: : :\n---\n\nbody";
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path, content.as_bytes().to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let outcome = store.reconcile(
        &collection,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;

    assert_eq!(outcome.malformed_frontmatter(), 1);

    let connection = Connection::open(&database_path)?;
    let passage_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM passage_files", [], |row| row.get(0))?;
    let kind: String =
        connection.query_row("SELECT kind FROM passage_files LIMIT 1", [], |row| {
            row.get(0)
        })?;
    assert_eq!(passage_count, 1);
    assert_eq!(kind, "body");

    Ok(())
}

/// Covers: FR-013 — an index-build failure rolls back file changes.
#[test]
fn reconcile_rolls_back_files_and_index_on_failure() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path.clone(), b"alpha".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TRIGGER fail_index_build
         BEFORE INSERT ON lexical_index_state
         BEGIN
             SELECT RAISE(ABORT, 'forced index failure');
         END;",
    )?;
    drop(connection);

    let error = store
        .reconcile(
            &collection,
            &[FileRecord::new(path.clone(), b"changed content".to_vec())],
            &[],
            Timestamp::from_unix_seconds(1_700_000_001),
        )
        .err()
        .ok_or_else(|| std::io::Error::other("a forced index failure should fail"))?;

    assert!(matches!(error, FileStoreError::Storage(_)));

    let connection = Connection::open(&database_path)?;
    let content: Vec<u8> =
        connection.query_row("SELECT content FROM files LIMIT 1", [], |row| row.get(0))?;
    let passage_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM passage_files", [], |row| row.get(0))?;
    let state_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM lexical_index_state", [], |row| {
            row.get(0)
        })?;

    assert_eq!(content, b"alpha".to_vec());
    assert_eq!(passage_count, 0);
    assert_eq!(state_count, 0);

    Ok(())
}

/// Covers: FR-009 and FR-010 — status reports `NotBuilt` on an unmigrated DB.
#[test]
fn status_reports_not_built_for_unmigrated_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");

    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TABLE schema_version (version INTEGER NOT NULL);
         INSERT INTO schema_version(version) VALUES (2);
         CREATE TABLE collections (
             collection_id INTEGER PRIMARY KEY,
             display_name TEXT NOT NULL,
             name_key TEXT NOT NULL UNIQUE,
             created_at INTEGER NOT NULL
         );
         CREATE TABLE files (
             file_id INTEGER PRIMARY KEY,
             collection_id INTEGER NOT NULL,
             path TEXT NOT NULL,
             content BLOB NOT NULL,
             content_hash TEXT NOT NULL,
             byte_size INTEGER NOT NULL,
             created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL,
             UNIQUE(collection_id, path)
         );
         INSERT INTO collections VALUES (1, 'Notes', 'notes', 0);
         INSERT INTO files VALUES (1, 1, '/a.md', X'61', 'hash', 1, 0, 0);",
    )?;
    drop(connection);

    let status_store = SqliteLexicalIndexStore::open(&database_path)?;
    let statuses = status_store.status()?;

    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;
    assert_eq!(status.state(), IndexState::NotBuilt);
    assert_eq!(status.file_count(), 1);
    assert_eq!(status.passage_count(), 0);
    assert_eq!(status.built_at(), None);

    Ok(())
}

/// Covers: FR-015 — a database with no collections reports nothing.
#[test]
fn status_reports_nothing_for_no_collections() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    SqliteCollectionStore::open(&database_path)?;

    let status_store = SqliteLexicalIndexStore::open(&database_path)?;
    let statuses = status_store.status()?;

    assert!(statuses.is_empty());

    Ok(())
}

/// Covers: FR-014 — the status store reports a missing database.
#[test]
fn lexical_index_store_reports_missing_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");

    let error = SqliteLexicalIndexStore::open(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(matches!(
        error,
        kv_application::CollectionStoreError::DatabaseNotFound
    ));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-002 — upserting files alone never builds the index.
#[test]
fn upserting_alone_does_not_build_the_index() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let path = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &collection,
        &[FileRecord::new(path, b"alpha\n\nbeta".to_vec())],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let connection = Connection::open(&database_path)?;
    let passage_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM passage_files", [], |row| row.get(0))?;
    let state_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM lexical_index_state", [], |row| {
            row.get(0)
        })?;

    assert_eq!(passage_count, 0);
    assert_eq!(state_count, 0);

    Ok(())
}
