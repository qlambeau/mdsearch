//! Integration tests for the `SQLite` hybrid search store.

use std::error::Error;
use std::path::Path;

use kv_application::{
    CollectionStore, FileRecord, FileStore, HybridSearchStore, HybridSearchStoreError, SearchScope,
    SemanticIndexStore,
};
use kv_domain::{CollectionName, Embedding, EmbeddingModel, PassageKind, Timestamp};
use rusqlite::Connection;
use tempfile::tempdir;

use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileStore, SqliteHybridSearchStore, SqliteSemanticIndexStore,
};

fn name(raw: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(raw)
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_seconds(1_700_000_000)
}

fn model(raw: &str) -> Result<EmbeddingModel, kv_domain::EmbeddingModelError> {
    EmbeddingModel::try_new(raw)
}

fn embedding() -> Embedding {
    Embedding::new(vec![0.1; 384])
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

/// Embeds one passage per file with the global model recorded.
fn embed(directory: &Path, collection: &CollectionName) -> Result<(), Box<dyn Error>> {
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.join("collections.db"))?;
    store.set_global_model(&model("all-MiniLM-L6-v2")?)?;
    let passages = store.passages(collection)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, embedding()))
        .collect::<Vec<_>>();
    store.rebuild(collection, &model("all-MiniLM-L6-v2")?, timestamp(), &pairs)?;

    Ok(())
}

/// Covers: FR-007 — a collection without a semantic index contributes lexically.
#[test]
fn a_collection_without_a_semantic_index_contributes_lexically() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing rules")])?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates = store.candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10)?;

    assert!(!candidates.lexical().is_empty());
    assert!(candidates.semantic().is_empty());

    Ok(())
}

/// Covers: FR-001 — a fully embedded collection contributes both legs.
#[test]
fn an_embedded_collection_contributes_both_legs() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing rules")])?;
    embed(directory.path(), &notes)?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates = store.candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10)?;

    assert!(!candidates.lexical().is_empty());
    assert!(!candidates.semantic().is_empty());

    Ok(())
}

/// Covers: REQ-011 FR-019 — a recorded dimension disagreeing with the active
/// dimension fails the command before any results are returned.
#[test]
fn dimension_mismatch_fails_before_results() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing rules")])?;
    embed(directory.path(), &notes)?;

    let connection = Connection::open(directory.path().join("collections.db"))?;
    connection.execute(
        "INSERT INTO settings(key, value) VALUES ('embedding_dimension', '1024')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [],
    )?;
    drop(connection);

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let error = store
        .candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10)
        .err()
        .ok_or_else(|| std::io::Error::other("a dimension mismatch should fail"))?;

    assert!(
        matches!(error, HybridSearchStoreError::DimensionMismatch { .. }),
        "unexpected error: {error:?}"
    );

    Ok(())
}

/// Covers: FR-010 — a stale semantic index fails the whole search.
#[test]
fn a_stale_semantic_index_fails() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;
    embed(directory.path(), &notes)?;

    let mut file_store =
        SqliteFileStore::open_for_ingestion(&directory.path().join("collections.db"))?;
    let added = FileRecord::new(
        directory.path().join("b.md"),
        "new file".as_bytes().to_vec(),
    );
    file_store.upsert_files(&notes, &[added], timestamp())?;
    file_store.reconcile(&notes, &[], &[], timestamp())?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;

    assert!(matches!(
        store.candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10),
        Err(HybridSearchStoreError::StaleSemanticIndex)
    ));

    Ok(())
}

/// Covers: FR-008 — unbuilt and empty collections are skipped in all mode.
#[test]
fn unbuilt_and_empty_collections_are_skipped_in_all_mode() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let empty = name("Empty")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;
    build(directory.path(), &empty, &[])?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates = store.candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10)?;

    assert!(!candidates.lexical().is_empty());

    Ok(())
}

/// Covers: FR-009 — a targeted unknown collection fails.
#[test]
fn a_targeted_unknown_collection_fails() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let journal = name("Journal")?;

    assert!(matches!(
        store.candidates(
            "\"borrowing\"",
            Some(&embedding()),
            SearchScope::Collection(&journal),
            10
        ),
        Err(HybridSearchStoreError::CollectionNotFound)
    ));

    Ok(())
}

/// Covers: FR-009 — a targeted collection without a built lexical index fails.
#[test]
fn a_targeted_unbuilt_lexical_index_fails() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let mut collections = SqliteCollectionStore::open(&directory.path().join("collections.db"))?;
    collections.create_collection(&notes, timestamp())?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;

    assert!(matches!(
        store.candidates(
            "\"borrowing\"",
            Some(&embedding()),
            SearchScope::Collection(&notes),
            10
        ),
        Err(HybridSearchStoreError::IndexNotBuilt)
    ));

    Ok(())
}

/// Covers: FR-010 — staleness is scoped to a targeted collection.
#[test]
fn a_stale_semantic_index_fails_for_a_targeted_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;
    embed(directory.path(), &notes)?;

    let mut file_store =
        SqliteFileStore::open_for_ingestion(&directory.path().join("collections.db"))?;
    let added = FileRecord::new(
        directory.path().join("b.md"),
        "new file".as_bytes().to_vec(),
    );
    file_store.upsert_files(&notes, &[added], timestamp())?;
    file_store.reconcile(&notes, &[], &[], timestamp())?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;

    assert!(matches!(
        store.candidates(
            "\"borrowing\"",
            Some(&embedding()),
            SearchScope::Collection(&notes),
            10
        ),
        Err(HybridSearchStoreError::StaleSemanticIndex)
    ));

    Ok(())
}

/// Covers: FR-003 — --collection restricts the semantic leg to one collection.
#[test]
fn a_targeted_collection_restricts_candidates() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let archive = name("Archive")?;
    build(directory.path(), &notes, &[("a.md", "borrowing rules")])?;
    build(
        directory.path(),
        &archive,
        &[("b.md", "borrowing anywhere")],
    )?;
    embed(directory.path(), &notes)?;
    embed(directory.path(), &archive)?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates = store.candidates(
        "\"borrowing\"",
        Some(&embedding()),
        SearchScope::Collection(&notes),
        10,
    )?;

    assert!(!candidates.lexical().is_empty());
    assert!(!candidates.semantic().is_empty());
    assert!(
        candidates
            .lexical()
            .iter()
            .all(|candidate| candidate.collection().display_name() == "Notes")
    );
    assert!(
        candidates
            .semantic()
            .iter()
            .all(|candidate| candidate.collection().display_name() == "Notes")
    );

    Ok(())
}

/// Covers: FR-015 — no matches produce empty candidate lists.
#[test]
fn no_matches_produce_empty_candidates() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;
    embed(directory.path(), &notes)?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates =
        store.candidates("\"zzznotaword\"", Some(&embedding()), SearchScope::All, 10)?;

    assert!(candidates.lexical().is_empty());
    assert!(!candidates.semantic().is_empty());

    Ok(())
}

/// Covers: FR-018 — a missing database fails without creating a file.
#[test]
fn a_missing_database_fails_without_creating_a_file() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing").join("collections.db");

    assert!(matches!(
        SqliteHybridSearchStore::open(&database_path),
        Err(kv_application::CollectionStoreError::DatabaseNotFound)
    ));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-017 — the global model settings are read from the store.
#[test]
fn global_model_settings_are_read() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing")])?;
    embed(directory.path(), &notes)?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;

    assert_eq!(
        store.global_model()?.map(|value| value.as_str().to_owned()),
        Some("all-MiniLM-L6-v2".to_owned())
    );
    assert!(store.reranker_model()?.is_none());

    Ok(())
}

/// Covers: FR-014 — candidates expose their passage identity and provenance.
#[test]
fn candidates_expose_passage_identity_and_provenance() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "borrowing rules")])?;
    embed(directory.path(), &notes)?;

    let store = SqliteHybridSearchStore::open(&directory.path().join("collections.db"))?;
    let candidates = store.candidates("\"borrowing\"", Some(&embedding()), SearchScope::All, 10)?;

    let candidate = candidates
        .lexical()
        .first()
        .ok_or_else(|| std::io::Error::other("expected a lexical candidate"))?;
    assert_eq!(candidate.collection().display_name(), "Notes");
    assert_eq!(candidate.kind(), PassageKind::Body);
    assert_eq!(candidate.text(), "borrowing rules");

    Ok(())
}
