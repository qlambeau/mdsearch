//! Integration tests for the `SQLite` semantic index store.

use std::error::Error;
use std::path::Path;

use kv_application::{
    CollectionIndexUpdate, CollectionStore, FileRecord, FileRetrievalStore, FileStore,
    PreparedSemanticCollection, PreparedSemanticPassage, SemanticIndexStore,
};
use kv_domain::{
    CollectionName, ContentHash, Embedding, EmbeddingModel, FileId, PassageKind, SemanticPassage,
    Timestamp,
};
use rusqlite::Connection;
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};
use tempfile::tempdir;

use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileRetrievalStore, SqliteFileStore, SqliteSemanticIndexStore,
};

fn open_with_vector(path: &Path) -> Result<Connection, Box<dyn Error>> {
    let connection = Connection::open(path)?;
    let extension_connection = ExtensionConnection::from_rusqlite(&connection);
    let module = StandardModule::<VectorTable<'_>>::new()
        .with_update()
        .with_transactions()
        .with_find_function();
    let registry = Registry::default();
    extension_connection.create_module("vector", module, registry.clone())?;
    scalar::register_scalar_functions(extension_connection, registry)?;
    Ok(connection)
}

fn name(raw: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(raw)
}

fn timestamp() -> Timestamp {
    Timestamp::from_unix_seconds(1_700_000_000)
}

fn model(raw: &str) -> Result<EmbeddingModel, kv_domain::EmbeddingModelError> {
    EmbeddingModel::try_new(raw)
}

fn passage(
    file: u64,
    kind: PassageKind,
    position: usize,
    text: &str,
) -> Result<SemanticPassage, kv_domain::FileIdError> {
    Ok(SemanticPassage::new(
        FileId::try_new(file)?,
        kind,
        position,
        text.to_owned(),
    ))
}

fn embedding() -> Embedding {
    Embedding::new(vec![0.1; 384])
}

fn embedding_of_dimension(dimension: usize) -> Embedding {
    Embedding::new(vec![0.1; dimension])
}

/// Reads the dimension declared in the stored `embeddings` table definition.
fn table_dimension(database_path: &Path) -> Result<i64, Box<dyn Error>> {
    let connection = Connection::open(database_path)?;
    let create: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'embeddings'",
        [],
        |row| row.get(0),
    )?;
    let value = create
        .split("dim=")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .ok_or_else(|| std::io::Error::other(format!("no dim in {create}")))?;
    Ok(value)
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

/// Covers: REQ-006 — the global model starts unset and can be recorded.
#[test]
fn global_model_starts_unset_and_is_recorded() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "body")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    assert!(store.global_model()?.is_none());

    store.set_global_model(&model("all-MiniLM-L6-v2")?)?;

    assert_eq!(
        store.global_model()?.map(|value| value.as_str().to_owned()),
        Some("all-MiniLM-L6-v2".to_owned())
    );

    Ok(())
}

/// Covers: REQ-011 FR-017 — the re-ranker model setting round-trips.
#[test]
fn reranker_model_setting_round_trips() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "body")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    assert!(store.reranker_model()?.is_none());

    let reranker = kv_domain::RerankerModel::try_new("bge-reranker-base")?;
    store.set_reranker_model(&reranker)?;

    assert_eq!(
        store
            .reranker_model()?
            .map(|value| value.as_str().to_owned()),
        Some("bge-reranker-base".to_owned())
    );

    Ok(())
}

/// Covers: REQ-001 — a fresh collection has no semantic status and embeds.
#[test]
fn fresh_collection_has_no_status_and_embeds() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "body")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    assert!(store.status(&notes)?.is_none());

    let passages = store.passages(&notes)?;
    let count = store.rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &[])?;
    assert_eq!(count, 0);
    assert!(!passages.is_empty());

    let status = store
        .status(&notes)?
        .ok_or_else(|| std::io::Error::other("expected a semantic status after the rebuild"))?;
    assert_eq!(status.passage_count(), 0);
    assert_eq!(status.model().as_str(), "all-MiniLM-L6-v2");

    Ok(())
}

/// Covers: REQ-001 and REQ-002 — passages are read in logical identity order.
#[test]
fn passages_are_read_with_their_logical_identity() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one\n\ntwo")])?;
    let store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let passages = store.passages(&notes)?;

    assert_eq!(passages.len(), 2);
    assert!(
        passages
            .iter()
            .all(|passage| passage.kind() == PassageKind::Body)
    );
    assert_eq!(
        passages
            .iter()
            .map(SemanticPassage::text)
            .collect::<Vec<_>>(),
        vec!["one", "two"]
    );

    Ok(())
}

/// Covers: REQ-004 — the file-set fingerprint is stable per stored set.
#[test]
fn fingerprint_is_stable_and_changes_with_the_file_set() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one")])?;
    let store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let first = store.file_set_fingerprint(&notes)?;
    let second = store.file_set_fingerprint(&notes)?;
    assert_eq!(first, second);

    let mut file_store =
        SqliteFileStore::open_for_ingestion(&directory.path().join("collections.db"))?;
    let added = FileRecord::new(directory.path().join("b.md"), "two".as_bytes().to_vec());
    file_store.upsert_files(&notes, &[added], timestamp())?;
    file_store.reconcile(&notes, &[], &[], timestamp())?;

    let store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;
    let third = store.file_set_fingerprint(&notes)?;
    assert_ne!(first, third);

    Ok(())
}

/// Covers: REQ-005 — rebuild replaces the collection's vectors atomically.
#[test]
fn rebuild_replaces_vectors_and_updates_state() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one\n\ntwo")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let passages = store.passages(&notes)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, embedding()))
        .collect::<Vec<_>>();
    let count = store.rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &pairs)?;
    assert_eq!(count, 2);

    let status = store
        .status(&notes)?
        .ok_or_else(|| std::io::Error::other("status after rebuild"))?;
    assert_eq!(status.passage_count(), 2);

    let vectors = vector_count(&directory.path().join("collections.db"), "Notes")?;
    assert_eq!(vectors, 2);

    let count = store.rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &[])?;
    assert_eq!(count, 0);
    let status = store
        .status(&notes)?
        .ok_or_else(|| std::io::Error::other("status after second rebuild"))?;
    assert_eq!(status.passage_count(), 0);
    let vectors = vector_count(&directory.path().join("collections.db"), "Notes")?;
    assert_eq!(vectors, 0);

    Ok(())
}

/// Covers: REQ-007 — embedded collections are listed for a model switch.
#[test]
fn embedded_collections_lists_only_embedded_collections() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let archive = name("Archive")?;
    build(directory.path(), &notes, &[("a.md", "one")])?;
    build(directory.path(), &archive, &[("b.md", "two")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    store.rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &[])?;

    let embedded = store.embedded_collections()?;
    assert_eq!(embedded.len(), 1);
    let first = embedded
        .first()
        .ok_or_else(|| std::io::Error::other("expected one embedded collection"))?;
    assert_eq!(first.display_name(), "Notes");

    Ok(())
}

/// Covers: REQ-011 — a collection without a built lexical index is a skip target.
#[test]
fn targets_report_lexical_state_and_file_presence() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let mut collections = SqliteCollectionStore::open(&directory.path().join("collections.db"))?;
    collections.create_collection(&notes, timestamp())?;

    let store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let targets = store.targets()?;
    assert_eq!(targets.len(), 1);
    let target = targets
        .first()
        .ok_or_else(|| std::io::Error::other("expected one target"))?;
    assert_eq!(target.collection().display_name(), "Notes");
    assert!(!target.has_files());
    assert!(!target.lexical_built());

    let resolved = store.resolve(&notes)?;
    assert_eq!(resolved, *target);

    Ok(())
}

/// Covers: REQ-014 — an unknown collection fails to resolve.
#[test]
fn resolve_rejects_an_unknown_collection() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one")])?;
    let store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;
    let journal = name("Journal")?;

    assert!(matches!(
        store.resolve(&journal),
        Err(kv_application::SemanticIndexStoreError::CollectionNotFound)
    ));

    Ok(())
}

/// Covers: REQ-017 — a missing database fails without creating a file.
#[test]
fn missing_database_fails_without_creating_a_file() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing").join("collections.db");

    assert!(matches!(
        SqliteSemanticIndexStore::open_for_embedding(&database_path),
        Err(kv_application::CollectionStoreError::DatabaseNotFound)
    ));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: REQ-002 and REQ-005 — a wrong-dimension embedding is rejected.
#[test]
fn rebuild_rejects_a_wrong_dimension_vector() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let bad = Embedding::new(vec![0.1; 8]);
    let pairs = vec![(passage(1, PassageKind::Body, 0, "one")?, bad)];

    assert!(
        store
            .rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &pairs)
            .is_err()
    );
    assert!(store.status(&notes)?.is_none());

    Ok(())
}

/// Covers: REQ-010 FR-024 — the rebuild guard names the expected and actual
/// dimensions in its error.
#[test]
fn rebuild_guard_names_expected_and_actual_dimensions() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;

    let bad = Embedding::new(vec![0.1; 8]);
    let pairs = vec![(passage(1, PassageKind::Body, 0, "one")?, bad)];

    let error = store
        .rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &pairs)
        .err()
        .ok_or_else(|| std::io::Error::other("a wrong-dimension rebuild should fail"))?;
    let message = match error {
        kv_application::SemanticIndexStoreError::Storage(inner) => inner.to_string(),
        other @ kv_application::SemanticIndexStoreError::CollectionNotFound => {
            return Err(std::io::Error::other(format!("unexpected error: {other:?}")).into());
        }
    };

    assert!(
        message.contains("expected 384"),
        "unexpected error: {message}"
    );
    assert!(message.contains("got 8"), "unexpected error: {message}");

    Ok(())
}

/// Covers: REQ-015 FR-001/FR-022 — a 1024-dimension rebuild succeeds and
/// records the batch dimension in the collection state and the active
/// dimension in settings, with the vector table created at that dimension.
#[test]
fn rebuild_stores_the_batch_dimension_and_active_setting() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one\n\ntwo")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;
    store.ensure_dimension(1024)?;

    let passages = store.passages(&notes)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, embedding_of_dimension(1024)))
        .collect::<Vec<_>>();
    let count = store.rebuild(&notes, &model("bge-large-en-v1.5")?, timestamp(), &pairs)?;
    assert_eq!(count, 2);

    let database_path = directory.path().join("collections.db");
    let connection = open_with_vector(&database_path)?;
    let state_dimension: i64 = connection.query_row(
        "SELECT dimension FROM semantic_index_state
         WHERE collection_id = (SELECT collection_id FROM collections WHERE name_key = ?1)",
        [notes.name_key()],
        |row| row.get(0),
    )?;
    assert_eq!(state_dimension, 1024);
    let setting: String = connection.query_row(
        "SELECT value FROM settings WHERE key = 'embedding_dimension'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(setting, "1024");
    assert_eq!(table_dimension(&database_path)?, 1024);

    Ok(())
}

/// Covers: REQ-015 FR-003 — rebuilding under a different-dimension model
/// recreates the vector table at the new dimension and updates the recorded
/// state.
#[test]
fn rebuild_recreates_the_table_when_the_dimension_changes() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("a.md", "one\n\ntwo")])?;
    let mut store =
        SqliteSemanticIndexStore::open_for_embedding(&directory.path().join("collections.db"))?;
    store.ensure_dimension(1024)?;

    let passages = store.passages(&notes)?;
    let wide = passages
        .iter()
        .cloned()
        .map(|passage| (passage, embedding_of_dimension(1024)))
        .collect::<Vec<_>>();
    store.rebuild(&notes, &model("bge-large-en-v1.5")?, timestamp(), &wide)?;

    store.ensure_dimension(384)?;
    let narrow = passages
        .iter()
        .cloned()
        .map(|passage| (passage, embedding_of_dimension(384)))
        .collect::<Vec<_>>();
    store.rebuild(&notes, &model("all-MiniLM-L6-v2")?, timestamp(), &narrow)?;

    let database_path = directory.path().join("collections.db");
    let connection = open_with_vector(&database_path)?;
    let state_dimension: i64 = connection.query_row(
        "SELECT dimension FROM semantic_index_state
         WHERE collection_id = (SELECT collection_id FROM collections WHERE name_key = ?1)",
        [notes.name_key()],
        |row| row.get(0),
    )?;
    assert_eq!(state_dimension, 384);
    let setting: String = connection.query_row(
        "SELECT value FROM settings WHERE key = 'embedding_dimension'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(setting, "384");
    assert_eq!(table_dimension(&database_path)?, 384);
    let vector_count = vector_count(&database_path, "Notes")?;
    assert_eq!(vector_count, 2);

    Ok(())
}

/// Covers: REQ-022 FR-011 — model setting, dimension, and all vectors switch together.
#[test]
fn rebuild_all_commits_global_model_and_multiple_collection_vectors_atomically()
-> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let notes = name("Notes")?;
    let archive = name("Archive")?;
    build(directory.path(), &notes, &[("notes.md", "notes body")])?;
    build(
        directory.path(),
        &archive,
        &[("archive.md", "archive body")],
    )?;
    let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let old_model = model("all-MiniLM-L6-v2")?;
    store.set_global_model(&old_model)?;
    store.ensure_dimension(384)?;
    let note_passages = store.passages(&notes)?;
    let archive_passages = store.passages(&archive)?;
    store.rebuild(
        &notes,
        &old_model,
        timestamp(),
        &note_passages
            .iter()
            .cloned()
            .map(|p| (p, embedding()))
            .collect::<Vec<_>>(),
    )?;
    store.rebuild(
        &archive,
        &old_model,
        timestamp(),
        &archive_passages
            .iter()
            .cloned()
            .map(|p| (p, embedding()))
            .collect::<Vec<_>>(),
    )?;

    let next_model = model("bge-small-en-v1.5")?;
    let batches = [
        PreparedSemanticCollection {
            collection: notes.clone(),
            embeddings: note_passages
                .iter()
                .cloned()
                .map(|p| (p, embedding_of_dimension(8)))
                .collect(),
        },
        PreparedSemanticCollection {
            collection: archive.clone(),
            embeddings: archive_passages
                .iter()
                .cloned()
                .map(|p| (p, embedding_of_dimension(8)))
                .collect(),
        },
    ];

    store.rebuild_all(&next_model, None, timestamp(), &batches)?;

    assert_eq!(store.global_model()?, Some(next_model.clone()));
    assert_eq!(
        store.status(&notes)?.map(|status| status.dimension()),
        Some(8)
    );
    assert_eq!(
        store.status(&archive)?.map(|status| status.dimension()),
        Some(8)
    );
    assert_eq!(table_dimension(&database_path)?, 8);
    assert_eq!(vector_count(&database_path, "Notes")?, 1);
    assert_eq!(vector_count(&database_path, "Archive")?, 1);
    Ok(())
}

/// Covers: REQ-022 FR-011 — a vector write failure rolls back model and dimension changes.
#[test]
fn rebuild_all_rolls_back_global_model_and_dimension_when_vector_insert_fails()
-> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("notes.md", "notes body")])?;
    let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let old_model = model("all-MiniLM-L6-v2")?;
    store.set_global_model(&old_model)?;
    store.ensure_dimension(384)?;
    let passages = store.passages(&notes)?;
    let old_batch = passages
        .iter()
        .cloned()
        .map(|p| (p, embedding()))
        .collect::<Vec<_>>();
    store.rebuild(&notes, &old_model, timestamp(), &old_batch)?;
    let before_status = store.status(&notes)?;
    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TRIGGER reject_state_update BEFORE UPDATE ON semantic_index_state
         BEGIN SELECT RAISE(ABORT, 'injected semantic-state failure'); END;",
    )?;
    let next_model = model("bge-small-en-v1.5")?;
    let batch = PreparedSemanticCollection {
        collection: notes.clone(),
        embeddings: passages
            .iter()
            .cloned()
            .map(|p| (p, embedding_of_dimension(8)))
            .collect(),
    };

    assert!(
        store
            .rebuild_all(&next_model, None, timestamp(), &[batch])
            .is_err()
    );

    assert_eq!(store.global_model()?, Some(old_model));
    assert_eq!(store.status(&notes)?, before_status);
    assert_eq!(table_dimension(&database_path)?, 384);
    assert_eq!(vector_count(&database_path, "Notes")?, 1);
    Ok(())
}

/// Covers: REQ-022 FR-003 — a successful update removes vectors for a disabled collection.
#[test]
fn file_reconcile_removes_semantic_vectors_after_policy_is_disabled() -> Result<(), Box<dyn Error>>
{
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let notes = name("Notes")?;
    build(directory.path(), &notes, &[("notes.md", "notes body")])?;
    let mut semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let embedding_model = model("all-MiniLM-L6-v2")?;
    semantic.set_global_model(&embedding_model)?;
    semantic.ensure_dimension(384)?;
    let passages = semantic.passages(&notes)?;
    let vectors = passages
        .into_iter()
        .map(|passage| (passage, embedding()))
        .collect::<Vec<_>>();
    semantic.rebuild(&notes, &embedding_model, timestamp(), &vectors)?;
    semantic.set_semantic_enabled(&notes, false)?;
    assert_eq!(vector_count(&database_path, "Notes")?, 1);

    let mut files = SqliteFileStore::open_for_ingestion(&database_path)?;
    files.reconcile(&notes, &[], &[], timestamp())?;
    let semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;

    assert!(!semantic.semantic_enabled(&notes)?);
    assert!(semantic.status(&notes)?.is_none());
    assert_eq!(vector_count(&database_path, "Notes")?, 0);
    Ok(())
}

/// Covers: REQ-022 FR-004 — file, lexical, graph, vector, and semantic state commit together.
#[test]
fn reconcile_collection_commits_enabled_semantic_index_with_file_changes()
-> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let notes = name("Notes")?;
    let file_path = directory.path().join("notes.md");
    build(directory.path(), &notes, &[("notes.md", "old passage")])?;
    let mut semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let embedding_model = model("all-MiniLM-L6-v2")?;
    semantic.set_global_model(&embedding_model)?;
    semantic.ensure_dimension(384)?;
    let old_passages = semantic.passages(&notes)?;
    semantic.rebuild(
        &notes,
        &embedding_model,
        timestamp(),
        &old_passages
            .iter()
            .cloned()
            .map(|p| (p, embedding()))
            .collect::<Vec<_>>(),
    )?;
    semantic.set_semantic_enabled(&notes, true)?;
    let update = CollectionIndexUpdate {
        collection: notes.clone(),
        upsert: vec![FileRecord::new(file_path.clone(), b"new passage".to_vec())],
        delete: Vec::new(),
        semantic: vec![PreparedSemanticPassage::new(
            file_path.clone(),
            PassageKind::Body,
            0,
            embedding(),
        )],
        model: embedding_model,
    };

    semantic.reconcile_collection(&update, Timestamp::from_unix_seconds(1_700_000_001))?;
    let retrieval = SqliteFileRetrievalStore::open(&database_path)?;
    let file = retrieval
        .get_by_path(&notes, &file_path)?
        .ok_or("reconciled file should be stored")?;
    let refreshed = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let passages = refreshed.passages(&notes)?;

    assert_eq!(file.content(), b"new passage");
    assert_eq!(passages.len(), 1);
    assert_eq!(
        passages.first().map(SemanticPassage::text),
        Some("new passage")
    );
    assert_eq!(vector_count(&database_path, "Notes")?, 1);
    assert_eq!(
        refreshed
            .status(&notes)?
            .map(|status| status.passage_count()),
        Some(1)
    );
    Ok(())
}

/// Covers: REQ-022 FR-004 — semantic-state failure rolls back files and all collection indexes.
#[test]
fn reconcile_collection_rolls_back_file_lexical_graph_and_vectors_on_semantic_failure()
-> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let notes = name("Notes")?;
    let file_path = directory.path().join("notes.md");
    build(directory.path(), &notes, &[("notes.md", "old passage")])?;
    let mut semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let embedding_model = model("all-MiniLM-L6-v2")?;
    semantic.set_global_model(&embedding_model)?;
    semantic.ensure_dimension(384)?;
    let old_passages = semantic.passages(&notes)?;
    semantic.rebuild(
        &notes,
        &embedding_model,
        timestamp(),
        &old_passages
            .iter()
            .cloned()
            .map(|p| (p, embedding()))
            .collect::<Vec<_>>(),
    )?;
    semantic.set_semantic_enabled(&notes, true)?;
    let before_status = semantic.status(&notes)?;
    let connection = Connection::open(&database_path)?;
    connection.execute_batch(
        "CREATE TRIGGER reject_semantic_state BEFORE UPDATE ON semantic_index_state
         BEGIN SELECT RAISE(ABORT, 'injected state failure'); END;",
    )?;
    let update = CollectionIndexUpdate {
        collection: notes.clone(),
        upsert: vec![FileRecord::new(file_path.clone(), b"new passage".to_vec())],
        delete: Vec::new(),
        semantic: vec![PreparedSemanticPassage::new(
            file_path.clone(),
            PassageKind::Body,
            0,
            embedding(),
        )],
        model: embedding_model,
    };

    assert!(
        semantic
            .reconcile_collection(&update, Timestamp::from_unix_seconds(1_700_000_001))
            .is_err()
    );

    let retrieval = SqliteFileRetrievalStore::open(&database_path)?;
    let file = retrieval
        .get_by_path(&notes, &file_path)?
        .ok_or("old file should remain")?;
    let semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    assert_eq!(file.content(), b"old passage");
    assert_eq!(semantic.passages(&notes)?, old_passages);
    assert_eq!(semantic.status(&notes)?, before_status);
    assert_eq!(vector_count(&database_path, "Notes")?, 1);
    Ok(())
}

fn vector_count(database_path: &Path, collection: &str) -> Result<i64, Box<dyn Error>> {
    let connection = open_with_vector(database_path)?;
    let collection_id: i64 = connection.query_row(
        "SELECT collection_id FROM collections WHERE display_name = ?1 LIMIT 1",
        [collection],
        |row| row.get(0),
    )?;
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM embeddings WHERE collection_id = ?1",
        [collection_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Guards: `ContentHash` is reachable through the domain API used here.
#[test]
fn content_hash_is_constructible() {
    let hash = ContentHash::from_content(b"files");
    assert_eq!(hash.as_str().len(), 64);
}
