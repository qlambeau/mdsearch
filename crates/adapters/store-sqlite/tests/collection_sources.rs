//! Persistence contract tests for registered collection sources.

use std::error::Error;
use std::path::PathBuf;

use kv_application::CollectionSourceStore;
use kv_application::CollectionStore;
use kv_domain::{CollectionName, CollectionSource, SourceKind, Timestamp};
use kv_store_sqlite::SqliteCollectionStore;
use tempfile::tempdir;

/// Covers: REQ-021 FR-001, FR-003 — source sets persist and replacement is visible.
#[test]
fn persists_and_replaces_registered_sources() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database)?;
    let collection = CollectionName::try_from("Notes")?;
    let first = CollectionSource::new(PathBuf::from("/vault"), SourceKind::Directory);
    let second = CollectionSource::new(PathBuf::from("/archive"), SourceKind::Directory);

    store.create_collection_with_sources(
        &collection,
        Timestamp::from_unix_seconds(1_700_000_000),
        std::slice::from_ref(&first),
    )?;
    store.replace_collection_sources(&collection, std::slice::from_ref(&second))?;

    assert_eq!(store.collection_sources(&collection)?, vec![second]);

    Ok(())
}

/// Covers: REQ-021 FR-012 — schema-v0 through schema-v7 collections migrate without inferred sources.
#[test]
fn migrations_from_all_legacy_versions_do_not_infer_sources() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    for version in 0..=7 {
        let database = directory.path().join(format!("v{version}.db"));
        {
            let mut store = SqliteCollectionStore::open(&database)?;
            let collection = CollectionName::try_from("Legacy")?;
            store.create_collection_with_sources(
                &collection,
                Timestamp::from_unix_seconds(1_700_000_000),
                &[],
            )?;
        }
        {
            let connection = rusqlite::Connection::open(&database)?;
            connection.execute("DROP TABLE collection_sources", [])?;
            connection.execute("UPDATE schema_version SET version = ?1", [version])?;
        }

        let store = SqliteCollectionStore::open(&database)?;
        let collection = CollectionName::try_from("Legacy")?;
        assert!(store.collection_sources(&collection)?.is_empty());
        let summaries = store.list_collections_with_sources()?;
        assert!(
            summaries
                .first()
                .ok_or("expected migrated collection")?
                .sources
                .is_empty()
        );
    }

    Ok(())
}

/// Covers: REQ-021 FR-001, FR-002 — duplicate source writes preserve stored configuration.
#[test]
fn duplicate_sources_rollback_create_and_configure_transactions() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database)?;
    let collection = CollectionName::try_from("Notes")?;
    let source = CollectionSource::new(PathBuf::from("/vault"), SourceKind::Directory);
    let duplicate = [source.clone(), source.clone()];

    assert!(
        store
            .create_collection_with_sources(
                &collection,
                Timestamp::from_unix_seconds(1_700_000_000),
                &duplicate,
            )
            .is_err()
    );
    assert!(store.list_collections_with_sources()?.is_empty());
    store.create_collection_with_sources(
        &collection,
        Timestamp::from_unix_seconds(1_700_000_000),
        std::slice::from_ref(&source),
    )?;
    assert!(
        store
            .replace_collection_sources(&collection, &duplicate)
            .is_err()
    );
    assert_eq!(store.collection_sources(&collection)?, vec![source]);

    Ok(())
}

/// Covers: REQ-021 FR-016 — collection destruction removes its source rows.
#[test]
fn destroying_a_collection_removes_its_source_rows() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database = directory.path().join("collections.db");
    let mut store = SqliteCollectionStore::open(&database)?;
    let collection = CollectionName::try_from("Notes")?;
    let source = CollectionSource::new(PathBuf::from("/vault"), SourceKind::Directory);
    store.create_collection_with_sources(
        &collection,
        Timestamp::from_unix_seconds(1_700_000_000),
        std::slice::from_ref(&source),
    )?;

    store.destroy_collection(&collection)?;

    let connection = rusqlite::Connection::open(&database)?;
    let source_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM collection_sources", [], |row| {
            row.get(0)
        })?;
    assert_eq!(source_count, 0);
    Ok(())
}
