//! Integration tests for the `SQLite` lexical search store.

use std::error::Error;
use std::path::Path;

use kv_application::{
    CollectionStore, FileRecord, FileStore, LexicalSearchStore, SearchScope, SearchStoreError,
};
use kv_domain::{CollectionName, Timestamp};
use rusqlite::Connection;
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore, SqliteLexicalSearchStore};

fn name(raw: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(raw)
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_seconds(1_700_000_000)
}

fn build(
    directory: &Path,
    collection: &CollectionName,
    files: &[(&str, &str)],
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
    store.reconcile(collection, &[], &[], timestamp())?;

    Ok(())
}

/// Covers: FR-004 — results are ranked by score, highest first.
#[test]
fn ranks_higher_term_frequency_passages_first() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(
        directory.path(),
        &notes,
        &[
            ("a.md", "borrowing borrowing borrowing"),
            ("b.md", "borrowing"),
        ],
    )?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("borrowing", 10, SearchScope::All)?;

    let results = set.results();
    assert_eq!(results.len(), 2);
    assert_eq!(set.total(), 2);
    let first = results
        .first()
        .ok_or_else(|| std::io::Error::other("expected a first result"))?;
    let second = results
        .get(1)
        .ok_or_else(|| std::io::Error::other("expected a second result"))?;
    assert!(first.path().ends_with("a.md"));
    assert!(second.path().ends_with("b.md"));
    assert!(first.score() >= second.score());

    Ok(())
}

/// Covers: FR-005 — equal scores are ordered by collection, path, position.
#[test]
fn breaks_ties_by_collection_then_path_then_position() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let archive = name("Archive")?;
    let notes = name("Notes")?;
    build(directory.path(), &archive, &[("b.md", "borrowing")])?;
    build(
        directory.path(),
        &notes,
        &[("a.md", "borrowing"), ("c.md", "borrowing")],
    )?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("borrowing", 10, SearchScope::All)?;

    let paths = set
        .results()
        .iter()
        .map(|result| result.path().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 3);
    let first = paths.first().map(String::as_str).unwrap_or_default();
    let second = paths.get(1).map(String::as_str).unwrap_or_default();
    let third = paths.get(2).map(String::as_str).unwrap_or_default();
    assert!(first.ends_with("b.md"), "Archive b.md first: {paths:?}");
    assert!(second.ends_with("a.md"), "Notes a.md second: {paths:?}");
    assert!(third.ends_with("c.md"), "Notes c.md third: {paths:?}");

    Ok(())
}

/// Covers: FR-003 — the limit caps results and the total is reported.
#[test]
fn limits_results_and_reports_the_total() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(
        directory.path(),
        &notes,
        &[
            ("a.md", "borrowing"),
            ("b.md", "borrowing"),
            ("c.md", "borrowing"),
        ],
    )?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("borrowing", 2, SearchScope::All)?;

    assert_eq!(set.results().len(), 2);
    assert_eq!(set.total(), 3);

    Ok(())
}

/// Covers: FR-002 — the collection scope restricts results.
#[test]
fn restricts_results_to_a_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let archive = name("Archive")?;
    let notes = name("Notes")?;
    build(directory.path(), &archive, &[("a.md", "borrowing")])?;
    build(directory.path(), &notes, &[("b.md", "borrowing")])?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("borrowing", 10, SearchScope::Collection(&notes))?;

    assert_eq!(set.total(), 1);
    let single = set
        .results()
        .first()
        .ok_or_else(|| std::io::Error::other("expected one result"))?;
    assert_eq!(single.collection().display_name(), "Notes");

    Ok(())
}

/// Covers: FR-008 — an unknown collection is an error.
#[test]
fn reports_an_unknown_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let error = store
        .search("borrowing", 10, SearchScope::Collection(&name("Missing")?))
        .err()
        .ok_or_else(|| std::io::Error::other("an unknown collection should fail"))?;

    assert!(matches!(error, SearchStoreError::CollectionNotFound));

    Ok(())
}

/// Covers: FR-009 — an unbuilt collection is an error.
#[test]
fn reports_an_unbuilt_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let database_path = directory.path().join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&notes, timestamp())?;
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &notes,
        &[FileRecord::new(
            directory.path().join("a.md"),
            b"borrowing".to_vec(),
        )],
        timestamp(),
    )?;

    let search_store = SqliteLexicalSearchStore::open(&database_path)?;
    let error = search_store
        .search("borrowing", 10, SearchScope::Collection(&notes))
        .err()
        .ok_or_else(|| std::io::Error::other("an unbuilt collection should fail"))?;

    assert!(matches!(error, SearchStoreError::IndexNotBuilt));

    Ok(())
}

/// Covers: FR-007 — unbuilt collections are skipped when searching all.
#[test]
fn skips_unbuilt_collections_when_searching_all() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let draft = name("Draft")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;

    let database_path = directory.path().join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&draft, timestamp())?;
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        &draft,
        &[FileRecord::new(
            directory.path().join("d.md"),
            b"borrowing".to_vec(),
        )],
        timestamp(),
    )?;

    let search_store = SqliteLexicalSearchStore::open(&database_path)?;
    let set = search_store.search("borrowing", 10, SearchScope::All)?;

    assert_eq!(set.total(), 1);
    let single = set
        .results()
        .first()
        .ok_or_else(|| std::io::Error::other("expected one result"))?;
    assert_eq!(single.collection().display_name(), "Notes");

    Ok(())
}

/// Covers: FR-006 — exact-phrase queries match only the phrase.
#[test]
fn matches_an_exact_phrase() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(
        directory.path(),
        &notes,
        &[("a.md", "rust ownership"), ("b.md", "rust borrow")],
    )?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("\"rust ownership\"", 10, SearchScope::All)?;

    assert_eq!(set.total(), 1);
    let single = set
        .results()
        .first()
        .ok_or_else(|| std::io::Error::other("expected one result"))?;
    assert!(single.path().ends_with("a.md"));

    Ok(())
}

/// Covers: FR-006 — a malformed query fails with an invalid-query error.
#[test]
fn reports_a_malformed_query() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let error = store
        .search("a AND", 10, SearchScope::All)
        .err()
        .ok_or_else(|| std::io::Error::other("a malformed query should fail"))?;

    assert!(matches!(error, SearchStoreError::InvalidQuery { .. }));

    Ok(())
}

/// Covers: the pre-v3 boundary — nothing is built.
#[test]
fn treats_a_pre_v3_database_as_having_no_built_indexes() -> Result<(), Box<dyn Error>> {
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
         INSERT INTO collections VALUES (1, 'Notes', 'notes', 0);",
    )?;
    drop(connection);

    let store = SqliteLexicalSearchStore::open(&database_path)?;

    let all = store.search("borrowing", 10, SearchScope::All)?;
    assert!(all.results().is_empty());
    assert_eq!(all.total(), 0);

    let error = store
        .search("borrowing", 10, SearchScope::Collection(&name("Notes")?))
        .err()
        .ok_or_else(|| std::io::Error::other("an unbuilt collection should fail"))?;
    assert!(matches!(error, SearchStoreError::IndexNotBuilt));

    let unknown = store
        .search("borrowing", 10, SearchScope::Collection(&name("Missing")?))
        .err()
        .ok_or_else(|| std::io::Error::other("an unknown collection should fail"))?;
    assert!(matches!(unknown, SearchStoreError::CollectionNotFound));

    Ok(())
}

/// Covers: FR-013 — the search store reports a missing database.
#[test]
fn reports_a_missing_database() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("nested").join("collections.db");

    let error = SqliteLexicalSearchStore::open(&database_path)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(matches!(
        error,
        kv_application::CollectionStoreError::DatabaseNotFound
    ));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-010 — result records carry collection, path, kind, text, score.
#[test]
fn results_carry_collection_path_kind_text_and_score() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(
        directory.path(),
        &notes,
        &[(
            "a.md",
            "---\ntitle: Borrowing\ntags: [rust]\n---\n\nborrowing rules",
        )],
    )?;

    let store = SqliteLexicalSearchStore::open(&directory.path().join("collections.db"))?;
    let set = store.search("borrowing", 10, SearchScope::All)?;

    assert_eq!(set.total(), 2);
    assert!(
        set.results()
            .iter()
            .all(|result| result.text().to_lowercase().contains("borrowing"))
    );
    assert!(set.results().iter().all(|result| {
        result.path().ends_with("a.md")
            && result.collection().display_name() == "Notes"
            && result.score() >= 0.0
    }));

    Ok(())
}
