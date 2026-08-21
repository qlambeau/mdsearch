//! Integration tests for the `SQLite` collection destroy path.

use std::error::Error;
use std::path::Path;

use kv_application::{
    CollectionStore, CollectionStoreError, FileRecord, FileStore, SemanticIndexStore,
};
use kv_domain::{CollectionName, Embedding, EmbeddingModel, SemanticPassage, Timestamp};
use rusqlite::Connection;
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore, SqliteSemanticIndexStore};

/// Opens a connection with the vector module registered, so queries may touch
/// the `embeddings` virtual table.
fn open_registered(path: &Path) -> Result<Connection, Box<dyn Error>> {
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

/// Builds a fully indexed collection: files, lexical index, entity graph, and
/// semantic index, all with data.
fn build_fully_indexed(
    directory: &Path,
    collection: &CollectionName,
) -> Result<(), Box<dyn Error>> {
    let database_path = directory.join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(collection, timestamp())?;

    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let content = "---\ntitle: Alpha\ntags: [rust]\n---\n\none\n\n[x](b.md)";
    store.upsert_files(
        collection,
        &[FileRecord::new(
            directory.join("a.md"),
            content.as_bytes().to_vec(),
        )],
        timestamp(),
    )?;
    store.reconcile(collection, &[], &[], timestamp())?;
    drop(store);

    let mut semantic = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    semantic.ensure_dimension(384)?;
    let passages = semantic.passages(collection)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, Embedding::new(vec![0.1; 384])))
        .collect::<Vec<(SemanticPassage, Embedding)>>();
    semantic.rebuild(
        collection,
        &EmbeddingModel::try_new("all-MiniLM-L6-v2")?,
        timestamp(),
        &pairs,
    )?;

    Ok(())
}

/// Builds a collection that has stored files but no indexes or graph.
fn build_files_only(directory: &Path, collection: &CollectionName) -> Result<(), Box<dyn Error>> {
    let database_path = directory.join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(collection, timestamp())?;

    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        collection,
        &[FileRecord::new(directory.join("a.md"), b"one".to_vec())],
        timestamp(),
    )?;
    Ok(())
}

fn collection_id(connection: &Connection, display_name: &str) -> Result<i64, Box<dyn Error>> {
    let id: i64 = connection.query_row(
        "SELECT collection_id FROM collections WHERE display_name = ?1 LIMIT 1",
        [display_name],
        |row| row.get(0),
    )?;
    Ok(id)
}

fn count_by_collection(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<i64, Box<dyn Error>> {
    let count: i64 = connection.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE collection_id = ?1"),
        [id],
        |row| row.get(0),
    )?;
    Ok(count)
}

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

/// Covers: REQ-016 FR-001 — destroying a fully indexed collection removes
/// every row belonging to it from all per-collection tables.
#[test]
fn destroying_a_fully_indexed_collection_leaves_no_trace() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build_fully_indexed(directory.path(), &notes)?;
    let database_path = directory.path().join("collections.db");

    let connection = open_registered(&database_path)?;
    let id = collection_id(&connection, "Notes")?;
    let passage_rowids = connection
        .prepare("SELECT passage_rowid FROM passage_files WHERE collection_id = ?1")?
        .query_map([id], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    assert!(!passage_rowids.is_empty());
    drop(connection);

    let mut store = SqliteCollectionStore::open_existing(&database_path)?;
    store.destroy_collection(&notes)?;
    drop(store);

    let connection = open_registered(&database_path)?;
    for table in [
        "files",
        "passage_files",
        "embeddings",
        "nodes",
        "edges",
        "graph_state",
        "lexical_index_state",
        "semantic_index_state",
    ] {
        assert_eq!(
            count_by_collection(&connection, table, id)?,
            0,
            "orphaned rows remain in {table}"
        );
    }
    let remaining_passages: i64 = if passage_rowids.is_empty() {
        0
    } else {
        let placeholders = vec!["?"; passage_rowids.len()].join(", ");
        connection.query_row(
            &format!("SELECT COUNT(*) FROM passages WHERE rowid IN ({placeholders})"),
            rusqlite::params_from_iter(passage_rowids.iter()),
            |row| row.get(0),
        )?
    };
    assert_eq!(remaining_passages, 0, "orphaned FTS5 passages remain");
    let collections_remaining: i64 = connection.query_row(
        "SELECT COUNT(*) FROM collections WHERE name_key = ?1",
        [notes.name_key()],
        |row| row.get(0),
    )?;
    assert_eq!(collections_remaining, 0);

    Ok(())
}

/// Covers: REQ-016 FR-001 — destroying a files-only collection removes its files.
#[test]
fn destroying_a_files_only_collection_removes_its_files() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build_files_only(directory.path(), &notes)?;
    let database_path = directory.path().join("collections.db");

    let mut store = SqliteCollectionStore::open_existing(&database_path)?;
    store.destroy_collection(&notes)?;
    drop(store);

    let connection = open_registered(&database_path)?;
    let files_remaining: i64 = connection.query_row(
        "SELECT COUNT(*) FROM files
         WHERE collection_id = (SELECT collection_id FROM collections WHERE display_name = 'Notes')",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(files_remaining, 0);

    Ok(())
}

/// Covers: REQ-016 FR-004 — destroying one collection leaves other
/// collections' data fully intact.
#[test]
fn destroying_one_collection_leaves_others_data_intact() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    let archive = name("Archive")?;
    build_fully_indexed(directory.path(), &notes)?;
    build_fully_indexed(directory.path(), &archive)?;
    let database_path = directory.path().join("collections.db");

    let connection = open_registered(&database_path)?;
    let archive_id = collection_id(&connection, "Archive")?;
    let archive_counts = [
        (
            "files",
            count_by_collection(&connection, "files", archive_id)?,
        ),
        (
            "passage_files",
            count_by_collection(&connection, "passage_files", archive_id)?,
        ),
        (
            "embeddings",
            count_by_collection(&connection, "embeddings", archive_id)?,
        ),
        (
            "nodes",
            count_by_collection(&connection, "nodes", archive_id)?,
        ),
        (
            "edges",
            count_by_collection(&connection, "edges", archive_id)?,
        ),
        (
            "graph_state",
            count_by_collection(&connection, "graph_state", archive_id)?,
        ),
        (
            "lexical_index_state",
            count_by_collection(&connection, "lexical_index_state", archive_id)?,
        ),
        (
            "semantic_index_state",
            count_by_collection(&connection, "semantic_index_state", archive_id)?,
        ),
    ];
    for (table, count) in &archive_counts {
        assert!(*count > 0, "Archive should have rows in {table}");
    }
    drop(connection);

    let mut store = SqliteCollectionStore::open_existing(&database_path)?;
    store.destroy_collection(&notes)?;
    drop(store);

    let connection = open_registered(&database_path)?;
    for (table, count) in &archive_counts {
        assert_eq!(
            count_by_collection(&connection, table, archive_id)?,
            *count,
            "Archive rows changed in {table}"
        );
    }

    Ok(())
}

/// Covers: REQ-016 FR-002 — a storage failure mid-destroy rolls back and
/// leaves the collection and its data intact.
#[test]
fn a_failed_destroy_leaves_the_collection_intact() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let notes = name("Notes")?;
    build_fully_indexed(directory.path(), &notes)?;
    let database_path = directory.path().join("collections.db");

    let connection = open_registered(&database_path)?;
    connection.execute(
        "CREATE TRIGGER fail_destroy BEFORE DELETE ON files
         BEGIN SELECT RAISE(ABORT, 'injected destroy failure'); END",
        [],
    )?;
    drop(connection);

    let mut store = SqliteCollectionStore::open_existing(&database_path)?;
    let error = store
        .destroy_collection(&notes)
        .err()
        .ok_or_else(|| std::io::Error::other("an injected failure should fail the destroy"))?;
    assert!(matches!(error, CollectionStoreError::Storage(_)));

    let connection = open_registered(&database_path)?;
    let id = collection_id(&connection, "Notes")?;
    assert_eq!(count_by_collection(&connection, "files", id)?, 1);
    assert_eq!(count_by_collection(&connection, "nodes", id)?, 2);
    assert!(
        count_by_collection(&connection, "embeddings", id)? > 0,
        "vectors should remain after a failed destroy"
    );
    assert_eq!(
        count_by_collection(&connection, "semantic_index_state", id)?,
        1
    );
    let collections_remaining: i64 = connection.query_row(
        "SELECT COUNT(*) FROM collections WHERE name_key = ?1",
        [notes.name_key()],
        |row| row.get(0),
    )?;
    assert_eq!(collections_remaining, 1);

    Ok(())
}
