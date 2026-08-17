//! Integration tests for the `SQLite` file retrieval store.

use std::error::Error;

use kv_application::{
    CollectionStore, FileRecord, FileRetrievalStore, FileRetrievalStoreError, FileStore,
};
use kv_domain::{CollectionName, FileId, Timestamp};
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileRetrievalStore, SqliteFileStore};

fn name() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_seconds(1_700_000_000)
}

fn path_arg(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

fn build(
    directory: &std::path::Path,
    collection: &CollectionName,
    files: &[(String, &str)],
) -> Result<(), Box<dyn Error>> {
    let database_path = directory.join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(collection, timestamp())?;

    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let records = files
        .iter()
        .map(|(path, content)| FileRecord::new(directory.join(path), content.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    store.upsert_files(collection, &records, timestamp())?;

    Ok(())
}

fn retrieve(directory: &std::path::Path) -> Result<SqliteFileRetrievalStore, Box<dyn Error>> {
    Ok(SqliteFileRetrievalStore::open(
        &directory.join("collections.db"),
    )?)
}

/// Covers: FR-004 — lookup by exact canonical path.
#[test]
fn looks_up_by_exact_path() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    let file_path = directory.path().join("vault").join("notes.md");
    build(
        directory.path(),
        &collection,
        &[(path_arg(&file_path), "alpha")],
    )?;

    let store = retrieve(directory.path())?;
    let file = store
        .get_by_path(&collection, &file_path)?
        .ok_or_else(|| std::io::Error::other("expected a file"))?;

    assert_eq!(file.content(), b"alpha");

    Ok(())
}

/// Covers: FR-003 — lookup by indexing-assigned ID.
#[test]
fn looks_up_by_id() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    let first = directory.path().join("a.md");
    let second = directory.path().join("b.md");
    build(
        directory.path(),
        &collection,
        &[(path_arg(&first), "alpha"), (path_arg(&second), "beta")],
    )?;

    let store = retrieve(directory.path())?;
    let file = store
        .get_by_id(&collection, FileId::try_new(2)?)?
        .ok_or_else(|| std::io::Error::other("expected a file"))?;

    assert_eq!(file.content(), b"beta");

    Ok(())
}

/// Covers: FR-004 — basename lookup lists every match.
#[test]
fn lists_files_by_basename() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    let a = directory.path().join("a").join("x.md");
    let b = directory.path().join("b").join("x.md");
    build(
        directory.path(),
        &collection,
        &[(path_arg(&a), "one"), (path_arg(&b), "two")],
    )?;

    let store = retrieve(directory.path())?;
    let matches = store.list_by_basename(&collection, "x.md")?;

    assert_eq!(matches.len(), 2);

    Ok(())
}

/// Covers: FR-006 — a missing path returns none.
#[test]
fn returns_none_for_a_missing_path() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        directory.path(),
        &collection,
        &[("a.md".to_owned(), "alpha")],
    )?;

    let store = retrieve(directory.path())?;
    let missing = directory.path().join("missing.md");
    let file = store.get_by_path(&collection, &missing)?;

    assert!(file.is_none());

    Ok(())
}

/// Covers: FR-006 — a missing ID returns none.
#[test]
fn returns_none_for_a_missing_id() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        directory.path(),
        &collection,
        &[("a.md".to_owned(), "alpha")],
    )?;

    let store = retrieve(directory.path())?;
    let file = store.get_by_id(&collection, FileId::try_new(999)?)?;

    assert!(file.is_none());

    Ok(())
}

/// Covers: FR-002 — an unknown collection is an error.
#[test]
fn reports_an_unknown_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        directory.path(),
        &collection,
        &[("a.md".to_owned(), "alpha")],
    )?;

    let store = retrieve(directory.path())?;
    let missing = CollectionName::try_from("Missing")?;
    let error = store
        .get_by_path(&missing, &directory.path().join("a.md"))
        .err()
        .ok_or_else(|| std::io::Error::other("an unknown collection should fail"))?;

    assert!(matches!(error, FileRetrievalStoreError::CollectionNotFound));

    Ok(())
}

/// Covers: FR-007 — a missing database fails without creation.
#[test]
fn reports_a_missing_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");

    let error = SqliteFileRetrievalStore::open(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(matches!(
        error,
        kv_application::CollectionStoreError::DatabaseNotFound
    ));
    assert!(!database_path.exists());

    Ok(())
}
