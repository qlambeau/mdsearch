//! REQ-023 FR-022/023: stored-content inspection and legacy read contracts.

use kv_application::{
    CollectionSourceStore, CollectionStore, FileRecord, FileStore, IndexInspectionStore,
    IndexStoreError, SemanticIndexStore,
};
use kv_domain::{
    CollectionName, Embedding, EmbeddingModel, IndexEnablement, IndexFreshness, Timestamp,
};
use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileStore, SqliteIndexInspectionStore, SqliteSemanticIndexStore,
};
use rusqlite::Connection;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use tempfile::{TempDir, tempdir};

fn seeded() -> Result<(TempDir, PathBuf, CollectionName, PathBuf), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let collection = CollectionName::try_from("Notes")?;
    let file = dir.path().join("a.md");
    let at = Timestamp::from_unix_seconds(42);
    SqliteCollectionStore::open(&database)?.create_collection(&collection, at)?;
    SqliteFileStore::open_for_ingestion(&database)?.reconcile(
        &collection,
        &[FileRecord::new(file.clone(), b"alpha".to_vec())],
        &[],
        at,
    )?;
    Ok((dir, database, collection, file))
}

/// Covers: REQ-023 FR-023 — equal passage counts cannot conceal changed text.
#[test]
fn lexical_freshness_detects_same_count_edits() -> Result<(), Box<dyn Error>> {
    let (_dir, database, collection, file) = seeded()?;
    let store = SqliteIndexInspectionStore::open(&database)?;
    let initial = store.inspect(Some(&collection))?;
    assert_eq!(
        initial
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .freshness,
        Some(IndexFreshness::Current)
    );
    SqliteFileStore::open_for_ingestion(&database)?.upsert_files(
        &collection,
        &[FileRecord::new(file, b"omega".to_vec())],
        Timestamp::from_unix_seconds(43),
    )?;

    let result = store.inspect(Some(&collection))?;

    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .freshness,
        Some(IndexFreshness::Stale)
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .passage_count,
        1
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .built_at,
        Some(Timestamp::from_unix_seconds(42))
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .graph
            .freshness,
        Some(IndexFreshness::Stale)
    );
    Ok(())
}

/// Covers: REQ-023 FR-023 — incomplete lexical mappings are stale.
fn lexical_freshness_detects_incomplete_mapping(mutation: &str) -> Result<(), Box<dyn Error>> {
    let (_dir, database, collection, _file) = seeded()?;
    Connection::open(&database)?.execute_batch(mutation)?;

    let report = SqliteIndexInspectionStore::open(&database)?.inspect(Some(&collection))?;

    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .freshness,
        Some(IndexFreshness::Stale)
    );
    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .built_at,
        Some(Timestamp::from_unix_seconds(42))
    );
    Ok(())
}

fn downgrade(database: &Path, version: i64) -> Result<(), Box<dyn Error>> {
    let connection = Connection::open(database)?;
    connection.execute("UPDATE schema_version SET version = ?1", [version])?;
    if version < 9 {
        connection.execute_batch("ALTER TABLE collections DROP COLUMN semantic_enabled;")?;
    }
    if version < 8 {
        connection.execute_batch("DROP TABLE collection_sources;")?;
    }
    if version < 7 {
        connection.execute_batch("ALTER TABLE semantic_index_state DROP COLUMN dimension;")?;
    }
    if version < 6 {
        connection.execute_batch("DROP TABLE edges; DROP TABLE nodes; DROP TABLE graph_state;")?;
    }
    if version < 5 {
        connection.execute_batch("DROP TABLE settings; DROP TABLE semantic_index_state;")?;
    }
    if version < 4 {
        connection.execute_batch("ALTER TABLE passage_files DROP COLUMN byte_offset;")?;
    }
    if version < 3 {
        connection.execute_batch(
            "DROP TABLE passage_files; DROP TABLE passages; DROP TABLE lexical_index_state;",
        )?;
    }
    if version < 2 {
        connection.execute_batch("DROP TABLE files;")?;
    }
    Ok(())
}

/// Covers: REQ-023 FR-023/027 — inspection reads legacy schemas without migration.
fn legacy_inspection_does_not_migrate(
    version: i64,
    expected: Option<IndexFreshness>,
) -> Result<(), Box<dyn Error>> {
    let (_dir, database, collection, _file) = seeded()?;
    downgrade(&database, version)?;
    let before = fs::read(&database)?;

    let report = SqliteIndexInspectionStore::open(&database)?.inspect(Some(&collection))?;
    let summaries =
        SqliteCollectionStore::open_existing(&database)?.list_collections_with_sources()?;

    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .freshness,
        expected
    );
    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .enablement,
        IndexEnablement::Disabled
    );
    assert_eq!(
        summaries
            .first()
            .ok_or("one listed collection required")?
            .name,
        collection
    );
    assert_eq!(fs::read(database)?, before);
    Ok(())
}

/// Covers: REQ-023 FR-022/023 — semantic state retains build and policy metadata.
#[test]
fn semantic_inspection_distinguishes_policy_freshness_and_model_compatibility()
-> Result<(), Box<dyn Error>> {
    let (_dir, database, collection, file) = seeded()?;
    let mut semantic = SqliteSemanticIndexStore::open_for_embedding(&database)?;
    semantic.set_semantic_enabled(&collection, true)?;
    let passages = semantic
        .passages(&collection)?
        .into_iter()
        .map(|passage| (passage, Embedding::new(vec![0.1; 384])))
        .collect::<Vec<_>>();
    semantic.rebuild(
        &collection,
        &EmbeddingModel::try_new("all-MiniLM-L6-v2")?,
        Timestamp::from_unix_seconds(43),
        &passages,
    )?;
    let initial = SqliteIndexInspectionStore::open(&database)?.inspect(None)?;
    assert_eq!(
        initial
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .freshness,
        Some(IndexFreshness::Current)
    );
    assert!(
        initial
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .compatible
    );
    semantic.set_global_model(&EmbeddingModel::try_new("bge-large-en-v1.5")?)?;
    let mismatched = SqliteIndexInspectionStore::open(&database)?.inspect(None)?;
    assert!(
        !mismatched
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .compatible
    );
    semantic.set_semantic_enabled(&collection, false)?;
    SqliteFileStore::open_for_ingestion(&database)?.upsert_files(
        &collection,
        &[FileRecord::new(file, b"changed".to_vec())],
        Timestamp::from_unix_seconds(44),
    )?;

    let result = SqliteIndexInspectionStore::open(&database)?.inspect(None)?;

    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .enablement,
        IndexEnablement::Disabled
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .freshness,
        Some(IndexFreshness::Stale)
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .dimension,
        Some(384)
    );
    assert_eq!(
        result
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .semantic
            .built_at,
        Some(Timestamp::from_unix_seconds(43))
    );
    Ok(())
}

/// Covers: REQ-023 FR-022/027 — empty and unknown scopes preserve read boundaries.
#[test]
fn inspection_errors_preserve_read_boundaries() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    assert!(SqliteIndexInspectionStore::open(&database).is_err());
    assert!(!database.exists());
    SqliteCollectionStore::open(&database)?;
    let store = SqliteIndexInspectionStore::open(&database)?;

    let report = store.inspect(None)?;

    assert!(report.collections.is_empty());
    assert!(matches!(
        store.inspect(Some(&CollectionName::try_from("missing")?)),
        Err(IndexStoreError::CollectionNotFound)
    ));
    Connection::open(&database)?.execute_batch("DROP TABLE collections;")?;
    assert!(matches!(
        store.inspect(None),
        Err(IndexStoreError::Storage(_))
    ));
    Ok(())
}

/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn missing_mapping() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping("DELETE FROM passage_files")
}
/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn wrong_position() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping("UPDATE passage_files SET position = 9")
}
/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn wrong_offset() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping("UPDATE passage_files SET byte_offset = 9")
}
/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn wrong_kind() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping("UPDATE passage_files SET kind = 'title'")
}
/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn extra_mapping() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping(
        "INSERT INTO passage_files SELECT passage_rowid + 999, collection_id, file_id, kind, position, byte_offset FROM passage_files",
    )
}
/// Covers: REQ-023 FR-023 — incomplete lexical storage is stale.
#[test]
fn missing_fts_row() -> Result<(), Box<dyn Error>> {
    lexical_freshness_detects_incomplete_mapping("DELETE FROM passages")
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v0() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(0, None)
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v1() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(1, None)
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v2() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(2, None)
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v3() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(3, Some(IndexFreshness::Current))
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v4() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(4, Some(IndexFreshness::Current))
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v5() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(5, Some(IndexFreshness::Current))
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v6() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(6, Some(IndexFreshness::Current))
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v7() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(7, Some(IndexFreshness::Current))
}
/// Covers: REQ-023 FR-027 — legacy schema reads do not migrate.
#[test]
fn legacy_v8() -> Result<(), Box<dyn Error>> {
    legacy_inspection_does_not_migrate(8, Some(IndexFreshness::Current))
}

/// Covers: REQ-023 FR-023/027 — legacy status detects stale text without backfilling.
#[test]
fn legacy_stale_index_is_not_certified_by_inspection() -> Result<(), Box<dyn Error>> {
    let (_dir, database, collection, file) = seeded()?;
    SqliteFileStore::open_for_ingestion(&database)?.upsert_files(
        &collection,
        &[FileRecord::new(file, b"omega".to_vec())],
        Timestamp::from_unix_seconds(43),
    )?;
    downgrade(&database, 3)?;
    let before = fs::read(&database)?;
    let report = SqliteIndexInspectionStore::open(&database)?.inspect(Some(&collection))?;
    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .freshness,
        Some(IndexFreshness::Stale)
    );
    assert_eq!(
        report
            .collections
            .first()
            .ok_or("one inspected collection required")?
            .lexical
            .built_at,
        Some(Timestamp::from_unix_seconds(42))
    );
    assert_eq!(fs::read(database)?, before);
    Ok(())
}
