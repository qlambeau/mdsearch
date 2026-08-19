//! `SQLite` implementation of the entity-graph read port.

use std::error::Error;
use std::path::Path;

use kv_application::{GraphStore, GraphStoreError, Neighbor};
use kv_domain::{CollectionName, EntityKind, GraphNode, NodeId, RelationKind};
use rusqlite::{Connection, OptionalExtension, params};

use crate::CollectionStoreError;

/// Reads an entity graph from the `nodes`/`edges` tables of a `SQLite` database.
pub struct SqliteGraphStore {
    connection: Connection,
}

impl SqliteGraphStore {
    /// Opens an existing database without creating or initializing it.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, or a
    /// database-unavailable error when it cannot be opened.
    pub fn open(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        Ok(Self { connection })
    }
}

impl GraphStore for SqliteGraphStore {
    fn node(
        &self,
        collection: &CollectionName,
        id: &NodeId,
    ) -> Result<Option<GraphNode>, GraphStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(graph_storage_failure)?
            .ok_or(GraphStoreError::CollectionNotFound)?;

        let row = self
            .connection
            .query_row(
                "SELECT node_kind, node_key, title FROM nodes
                 WHERE collection_id = ?1 AND node_kind = ?2 AND node_key = ?3
                 LIMIT 1",
                params![collection_id, id.kind().as_str(), id.key(),],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(graph_storage_failure)?;

        Ok(row.map(|(kind, key, title)| {
            let kind = EntityKind::from_key(&kind).unwrap_or(EntityKind::File);
            GraphNode::new(NodeId::new(kind, key), title)
        }))
    }

    fn neighbors(
        &self,
        collection: &CollectionName,
        id: &NodeId,
        relation: Option<RelationKind>,
        max_hops: u8,
    ) -> Result<Vec<Neighbor>, GraphStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(graph_storage_failure)?
            .ok_or(GraphStoreError::CollectionNotFound)?;

        let relation = relation.map(RelationKind::as_str);
        let mut statement = self
            .connection
            .prepare(
                "WITH RECURSIVE walk(node_id, depth, path, relation) AS (
                    SELECT e.dst_id, 1, CAST(e.src_id AS TEXT) || ',' || CAST(e.dst_id AS TEXT), e.relation
                    FROM edges e
                    JOIN nodes s ON e.src_id = s.node_id
                    WHERE s.collection_id = ?1
                      AND s.node_kind = ?2
                      AND s.node_key = ?3
                      AND (?4 IS NULL OR e.relation = ?4)
                    UNION ALL
                    SELECT e.dst_id, w.depth + 1, w.path || ',' || CAST(e.dst_id AS TEXT), e.relation
                    FROM edges e
                    JOIN walk w ON e.src_id = w.node_id
                    WHERE w.depth < ?5
                      AND instr(w.path, CAST(e.dst_id AS TEXT)) = 0
                      AND e.collection_id = ?1
                      AND (?4 IS NULL OR e.relation = ?4)
                )
                SELECT n.node_kind, n.node_key, n.title, w.relation, w.depth
                FROM walk w
                JOIN nodes n ON n.node_id = w.node_id
                WHERE n.collection_id = ?1
                ORDER BY w.depth, n.node_kind, n.node_key",
            )
            .map_err(graph_storage_failure)?;

        let rows = statement
            .query_map(
                params![
                    collection_id,
                    id.kind().as_str(),
                    id.key(),
                    relation,
                    max_hops,
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, u8>(4)?,
                    ))
                },
            )
            .map_err(graph_storage_failure)?;

        let mut neighbors = Vec::new();
        for row in rows {
            let (kind, key, title, relation, depth) = row.map_err(graph_storage_failure)?;
            let kind = EntityKind::from_key(&kind).unwrap_or(EntityKind::File);
            let node = GraphNode::new(NodeId::new(kind, key), title);
            let relation = RelationKind::from_key(&relation).unwrap_or(RelationKind::LinksTo);
            neighbors.push(Neighbor::new(node, relation, depth));
        }

        Ok(neighbors)
    }
}

impl SqliteGraphStore {
    fn resolve_collection_id(
        &self,
        collection: &CollectionName,
    ) -> Result<Option<i64>, rusqlite::Error> {
        self.connection
            .query_row(
                "SELECT collection_id FROM collections WHERE name_key = ?1 LIMIT 1",
                params![collection.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
    }
}

fn graph_storage_failure(error: impl Error + Send + Sync + 'static) -> GraphStoreError {
    GraphStoreError::Storage(Box::new(error))
}

fn database_unavailable(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::DatabaseUnavailable(Box::new(error))
}
