//! Integration tests for the entity graph build inside `reconcile`.

use std::error::Error;
use std::path::Path;

use kv_application::{CollectionStore, FileRecord, FileStore};
use kv_domain::{CollectionName, Timestamp};
use rusqlite::Connection;
use tempfile::tempdir;

use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore};

fn name() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

fn path_str(path: &std::path::Path) -> Result<&str, Box<dyn Error>> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("test path should be UTF-8").into())
}

fn node_count(connection: &Connection, key: &str, kind: &str) -> Result<i64, Box<dyn Error>> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM nodes WHERE node_key = ?1 AND node_kind = ?2",
        rusqlite::params![key, kind],
        |row| row.get(0),
    )?)
}

fn edge_count(
    connection: &Connection,
    relation: &str,
    src_key: &str,
    dst_key: &str,
) -> Result<i64, Box<dyn Error>> {
    Ok(connection.query_row(
        "SELECT COUNT(*) FROM edges e
         JOIN nodes s ON e.src_id = s.node_id
         JOIN nodes d ON e.dst_id = d.node_id
         WHERE e.relation = ?1 AND s.node_key = ?2 AND d.node_key = ?3",
        rusqlite::params![relation, src_key, dst_key],
        |row| row.get(0),
    )?)
}

fn upsert_and_reconcile(
    store: &mut SqliteFileStore,
    collection: &CollectionName,
    files: &[(&Path, &[u8])],
) -> Result<(), Box<dyn Error>> {
    let records = files
        .iter()
        .map(|(path, content)| FileRecord::new(path.to_path_buf(), content.to_vec()))
        .collect::<Vec<_>>();
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
fn reconcile_builds_graph_nodes_and_edges() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let b = directory.path().join("b.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[
            (a.as_path(), b"---\ntags: [rust]\n---\n[to](b.md)\n"),
            (b.as_path(), b"---\ntags: [rust]\n---\nbody\n"),
        ],
    )?;

    let connection = Connection::open(&database_path)?;
    assert_eq!(node_count(&connection, path_str(&a)?, "file")?, 1);
    assert_eq!(node_count(&connection, path_str(&b)?, "file")?, 1);
    assert_eq!(node_count(&connection, "rust", "tag")?, 1);
    assert_eq!(
        edge_count(&connection, "LINKS_TO", path_str(&a)?, path_str(&b)?)?,
        1
    );
    assert_eq!(
        edge_count(&connection, "TAGGED_WITH", path_str(&a)?, "rust")?,
        1
    );

    Ok(())
}

#[test]
fn reconcile_skips_unresolved_related_reference() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[(a.as_path(), b"---\nrelated: [missing]\n---\nbody\n")],
    )?;

    let connection = Connection::open(&database_path)?;
    let related: i64 = connection.query_row(
        "SELECT COUNT(*) FROM edges WHERE relation = 'RELATED_TO'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(related, 0);

    Ok(())
}

#[test]
fn reconcile_removes_stale_nodes_after_deletion() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let b = directory.path().join("b.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[
            (a.as_path(), b"---\n---\n[to](b.md)\n"),
            (b.as_path(), b"---\n---\nbody\n"),
        ],
    )?;

    let connection = Connection::open(&database_path)?;
    assert_eq!(node_count(&connection, path_str(&b)?, "file")?, 1);

    store.reconcile(
        &collection,
        &[],
        std::slice::from_ref(&b),
        Timestamp::from_unix_seconds(1_700_000_002),
    )?;

    assert_eq!(node_count(&connection, path_str(&b)?, "file")?, 0);
    assert_eq!(
        edge_count(&connection, "LINKS_TO", path_str(&a)?, path_str(&b)?)?,
        0
    );

    Ok(())
}

#[test]
fn reconcile_is_idempotent_for_unchanged_files() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let b = directory.path().join("b.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[
            (a.as_path(), b"---\ntags: [rust]\n---\n[to](b.md)\n"),
            (b.as_path(), b"---\n---\nbody\n"),
        ],
    )?;

    let connection = Connection::open(&database_path)?;
    let first_nodes: i64 =
        connection.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
    let first_edges: i64 =
        connection.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
    assert_eq!(first_nodes, 3);
    assert_eq!(first_edges, 2);

    store.reconcile(
        &collection,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_002),
    )?;
    let second_nodes: i64 =
        connection.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
    let second_edges: i64 =
        connection.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
    assert_eq!(first_nodes, second_nodes);
    assert_eq!(first_edges, second_edges);

    Ok(())
}

#[test]
fn reconcile_keeps_tag_and_alias_nodes_distinct() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[(a.as_path(), b"---\ntags: [mt]\naliases: [mt]\n---\nbody\n")],
    )?;

    let connection = Connection::open(&database_path)?;
    assert_eq!(node_count(&connection, "mt", "tag")?, 1);
    assert_eq!(node_count(&connection, "mt", "alias")?, 1);

    Ok(())
}

#[test]
fn reconcile_records_graph_state() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let b = directory.path().join("b.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[
            (a.as_path(), b"---\n---\n[to](b.md)\n"),
            (b.as_path(), b"---\n---\nbody\n"),
        ],
    )?;

    let connection = Connection::open(&database_path)?;
    let (node_count, edge_count): (i64, i64) = connection.query_row(
        "SELECT node_count, edge_count FROM graph_state WHERE collection_id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(node_count, 2);
    assert_eq!(edge_count, 1);

    Ok(())
}

#[test]
fn reconcile_on_unknown_collection_rolls_back_graph() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let collection = name()?;
    let mut collections = SqliteCollectionStore::open(&database_path)?;
    collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

    let a = directory.path().join("a.md");
    let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
    upsert_and_reconcile(
        &mut store,
        &collection,
        &[(a.as_path(), b"---\ntags: [rust]\n---\nbody\n")],
    )?;

    let connection = Connection::open(&database_path)?;
    let before: i64 = connection.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
    assert_eq!(before, 2);

    let ghost = CollectionName::try_from("ghost")?;
    let result = store.reconcile(
        &ghost,
        &[],
        &[],
        Timestamp::from_unix_seconds(1_700_000_002),
    );
    assert!(result.is_err());

    let after: i64 = connection.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;
    assert_eq!(after, before);

    Ok(())
}
