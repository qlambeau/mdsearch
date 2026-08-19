//! Integration tests for the `SQLite` graph read store.

use std::error::Error;

use kv_application::{CollectionStore, FileRecord, FileStore, GraphStore};
use kv_domain::{CollectionName, EntityKind, NodeId, RelationKind, Timestamp};
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore, SqliteGraphStore};

fn name() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

fn file_id(kind: EntityKind, key: &str) -> NodeId {
    NodeId::new(kind, key.to_owned())
}

fn path_str(path: &std::path::Path) -> Result<&str, Box<dyn Error>> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("test path should be UTF-8").into())
}

fn build(
    directory: &tempfile::TempDir,
    collection: &CollectionName,
    files: &[(&str, &[u8])],
) -> Result<(), Box<dyn Error>> {
    let database_path = directory.path().join("collections.db");
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let records = files
        .iter()
        .map(|(path, content)| FileRecord::new(directory.path().join(path), content.to_vec()))
        .collect::<Vec<_>>();
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    store.upsert_files(
        collection,
        &records,
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;
    store.reconcile(
        collection,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_001),
    )?;
    Ok(())
}

#[test]
fn node_lookup_finds_and_misses() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        &directory,
        &collection,
        &[
            ("a.md", b"---\n---\n[to](b.md)\n"),
            ("b.md", b"---\n---\nbody\n"),
        ],
    )?;

    let store = SqliteGraphStore::open(&directory.path().join("collections.db"))?;
    let a = file_id(EntityKind::File, path_str(&directory.path().join("a.md"))?);
    let found = store.node(&collection, &a)?;
    assert!(found.is_some());

    let missing = file_id(EntityKind::File, "zzz.md");
    assert!(store.node(&collection, &missing)?.is_none());

    Ok(())
}

#[test]
fn neighbors_filter_by_relation_and_depth() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        &directory,
        &collection,
        &[
            ("a.md", b"---\n---\n[to](b.md)\n"),
            ("b.md", b"---\n---\n[to](c.md)\n"),
            ("c.md", b"---\n---\nbody\n"),
        ],
    )?;

    let store = SqliteGraphStore::open(&directory.path().join("collections.db"))?;
    let a = file_id(EntityKind::File, path_str(&directory.path().join("a.md"))?);

    let one_hop = store.neighbors(&collection, &a, None, 1)?;
    let keys: Vec<&str> = one_hop.iter().map(|n| n.node().id().key()).collect();
    assert_eq!(keys.len(), 1);
    let first = one_hop.first().ok_or("expected a neighbor")?;
    assert!(first.node().id().key().ends_with("b.md"));
    assert_eq!(first.depth(), 1);

    let two_hops = store.neighbors(&collection, &a, None, 2)?;
    let mut keys: Vec<&str> = two_hops.iter().map(|n| n.node().id().key()).collect();
    keys.sort_unstable();
    assert_eq!(keys.len(), 2);

    let filtered = store.neighbors(&collection, &a, Some(RelationKind::LinksTo), 2)?;
    assert_eq!(filtered.len(), 2);

    let tagged = store.neighbors(&collection, &a, Some(RelationKind::TaggedWith), 2)?;
    assert!(tagged.is_empty());

    Ok(())
}

#[test]
fn unknown_collection_is_an_error() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(&directory, &collection, &[("a.md", b"body\n")])?;

    let store = SqliteGraphStore::open(&directory.path().join("collections.db"))?;
    let ghost = CollectionName::try_from("ghost")?;
    let a = file_id(EntityKind::File, "a.md");
    let result = store.node(&ghost, &a);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn cycle_terminates() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let collection = name()?;
    build(
        &directory,
        &collection,
        &[
            ("a.md", b"---\n---\n[to](b.md)\n"),
            ("b.md", b"---\n---\n[to](a.md)\n"),
        ],
    )?;

    let store = SqliteGraphStore::open(&directory.path().join("collections.db"))?;
    let a = file_id(EntityKind::File, path_str(&directory.path().join("a.md"))?);
    let neighbors = store.neighbors(&collection, &a, None, 5)?;
    let keys: Vec<&str> = neighbors.iter().map(|n| n.node().id().key()).collect();
    assert_eq!(keys.len(), 1);
    assert!(keys.first().is_some_and(|k| k.ends_with("b.md")));

    Ok(())
}
