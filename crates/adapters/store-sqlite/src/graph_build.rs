//! `SQLite` graph build persistence.

use std::path::{Path, PathBuf};

use kv_application::FileStoreError;
use kv_domain::{
    ContentHash, EntityGraph, FileId, GraphSource, extract_graph, file_set_fingerprint,
};
use rusqlite::{Transaction, params};

use crate::file_storage_failure;

/// Rebuilds the entity graph for a collection from its stored files.
///
/// Reads the current `files` rows, derives the deterministic [`EntityGraph`]
/// with [`extract_graph`], replaces the collection's `nodes`/`edges`, and
/// records `graph_state`. The caller runs this inside the collection's
/// transaction so a failure rolls back the whole reconcile.
pub(super) fn rebuild_graph(
    transaction: &Transaction<'_>,
    collection_id: i64,
    built_at: i64,
) -> Result<(), FileStoreError> {
    let files = read_graph_files(transaction, collection_id)?;
    let sources = to_graph_sources(&files)?;
    let graph = extract_graph(&sources);

    clear_graph(transaction, collection_id)?;
    insert_graph_nodes(transaction, collection_id, &graph)?;
    insert_graph_edges(transaction, collection_id, &graph)?;
    write_graph_state(transaction, collection_id, built_at, &files, &graph)?;

    Ok(())
}

/// Reads the `(file_id, path, content)` rows of a collection in id order.
pub(super) fn read_graph_files(
    transaction: &Transaction<'_>,
    collection_id: i64,
) -> Result<Vec<(i64, String, Vec<u8>)>, FileStoreError> {
    let mut statement = transaction
        .prepare(
            "SELECT file_id, path, content FROM files
             WHERE collection_id = ?1 ORDER BY file_id",
        )
        .map_err(file_storage_failure)?;
    let rows = statement
        .query_map(params![collection_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(file_storage_failure)?;
    let mut files = Vec::new();
    for row in rows {
        files.push(row.map_err(file_storage_failure)?);
    }
    Ok(files)
}

/// Converts stored file rows into [`GraphSource`] values.
pub(super) fn to_graph_sources(
    files: &[(i64, String, Vec<u8>)],
) -> Result<Vec<GraphSource<'_>>, FileStoreError> {
    files
        .iter()
        .map(|(file_id, path, content)| {
            let file_id = u64::try_from(*file_id).map_err(file_storage_failure)?;
            let file_id = FileId::try_new(file_id).map_err(file_storage_failure)?;
            Ok(GraphSource::new(file_id, Path::new(path), content))
        })
        .collect()
}

/// Deletes the collection's existing nodes and edges.
pub(super) fn clear_graph(
    transaction: &Transaction<'_>,
    collection_id: i64,
) -> Result<(), FileStoreError> {
    transaction
        .execute(
            "DELETE FROM edges WHERE collection_id = ?1",
            params![collection_id],
        )
        .map_err(file_storage_failure)?;
    transaction
        .execute(
            "DELETE FROM nodes WHERE collection_id = ?1",
            params![collection_id],
        )
        .map_err(file_storage_failure)?;
    Ok(())
}

/// Inserts the graph's nodes, returning the `(kind, key)` to node-id mapping.
pub(super) fn insert_graph_nodes(
    transaction: &Transaction<'_>,
    collection_id: i64,
    graph: &EntityGraph,
) -> Result<(), FileStoreError> {
    for node in graph.nodes() {
        let id = node.id();
        transaction
            .execute(
                "INSERT INTO nodes(collection_id, node_kind, node_key, title)
                 VALUES (?1, ?2, ?3, ?4)",
                params![collection_id, id.kind().as_str(), id.key(), node.title(),],
            )
            .map_err(file_storage_failure)?;
    }
    Ok(())
}

/// Inserts the graph's edges, resolving node ids by `(kind, key)`.
pub(super) fn insert_graph_edges(
    transaction: &Transaction<'_>,
    collection_id: i64,
    graph: &EntityGraph,
) -> Result<(), FileStoreError> {
    let mut node_ids: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    {
        let mut statement = transaction
            .prepare("SELECT node_id, node_kind, node_key FROM nodes WHERE collection_id = ?1")
            .map_err(file_storage_failure)?;
        let rows = statement
            .query_map(params![collection_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(file_storage_failure)?;
        for row in rows {
            let (node_id, kind, key) = row.map_err(file_storage_failure)?;
            node_ids.insert((kind, key), node_id);
        }
    }

    for edge in graph.edges() {
        let src = node_ids
            .get(&(
                edge.src().kind().as_str().to_owned(),
                edge.src().key().to_owned(),
            ))
            .copied()
            .ok_or_else(|| {
                FileStoreError::Storage("graph edge references unknown source node".into())
            })?;
        let dst = node_ids
            .get(&(
                edge.dst().kind().as_str().to_owned(),
                edge.dst().key().to_owned(),
            ))
            .copied()
            .ok_or_else(|| {
                FileStoreError::Storage("graph edge references unknown destination node".into())
            })?;
        transaction
            .execute(
                "INSERT INTO edges(collection_id, src_id, dst_id, relation)
                 VALUES (?1, ?2, ?3, ?4)",
                params![collection_id, src, dst, edge.relation().as_str()],
            )
            .map_err(file_storage_failure)?;
    }
    Ok(())
}

/// Records the collection's graph build state.
pub(super) fn write_graph_state(
    transaction: &Transaction<'_>,
    collection_id: i64,
    built_at: i64,
    files: &[(i64, String, Vec<u8>)],
    graph: &EntityGraph,
) -> Result<(), FileStoreError> {
    let node_count = i64::try_from(graph.node_count()).map_err(file_storage_failure)?;
    let edge_count = i64::try_from(graph.edge_count()).map_err(file_storage_failure)?;

    let hashes: Vec<(PathBuf, ContentHash)> = files
        .iter()
        .map(|(_, path, content)| {
            (
                Path::new(path).to_path_buf(),
                ContentHash::from_content(content),
            )
        })
        .collect();
    let fingerprint_refs: Vec<(&Path, &ContentHash)> = hashes
        .iter()
        .map(|(path, hash)| (path.as_path(), hash))
        .collect();
    let fingerprint = file_set_fingerprint(&fingerprint_refs);

    transaction
        .execute(
            "INSERT INTO graph_state(collection_id, file_set_fingerprint, node_count, edge_count, built_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(collection_id) DO UPDATE SET
                 file_set_fingerprint = excluded.file_set_fingerprint,
                 node_count = excluded.node_count,
                 edge_count = excluded.edge_count,
                 built_at = excluded.built_at",
            params![
                collection_id,
                fingerprint.as_str(),
                node_count,
                edge_count,
                built_at,
            ],
        )
        .map_err(file_storage_failure)?;

    Ok(())
}
