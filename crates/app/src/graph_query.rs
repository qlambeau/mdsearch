//! In-process `async_graphql` query layer over the entity graph.
//!
//! This is an internal, library-only surface (no GraphQL server is exposed).
//! It wraps the [`GraphStore`] port so future slices (EPIC-006) can drive
//! related-concept queries, and so the debug CLI and tests can exercise the
//! graph through a typed query.

use std::sync::Mutex;

use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema, SimpleObject};
use kv_application::{GraphStore, GraphStoreError, Neighbor};
use kv_domain::{CollectionName, EntityKind, NodeId, RelationKind};

/// A serializable view of a graph node for the query layer.
#[derive(SimpleObject)]
struct NodeOut {
    kind: String,
    key: String,
    title: String,
}

/// A serializable view of a neighbor for the query layer.
#[derive(SimpleObject)]
struct NeighborOut {
    kind: String,
    key: String,
    title: String,
    relation: String,
    depth: u8,
}

/// The graph query root exposing `node` and `neighbors`.
pub struct GraphQueryRoot<S> {
    store: Mutex<S>,
}

#[Object]
impl<S: GraphStore + Send + 'static> GraphQueryRoot<S> {
    /// Returns the node with the given kind and key.
    ///
    /// Errors when the node does not exist in the collection (REQ-013 FR-008).
    #[allow(clippy::unused_async)]
    async fn node(
        &self,
        ctx: &Context<'_>,
        kind: String,
        key: String,
    ) -> async_graphql::Result<NodeOut> {
        let collection = ctx.data::<CollectionName>()?;
        let id = NodeId::new(parse_kind(&kind)?, key);
        let guard = self
            .store
            .lock()
            .map_err(|_| async_graphql::Error::new("graph store lock poisoned"))?;
        let node = guard
            .node(collection, &id)
            .map_err(|error| map_error(&error))?
            .ok_or_else(|| node_not_found(&id))?;
        Ok(node.into())
    }

    /// Returns the neighbors of the node within `max_hops`, optionally filtered
    /// by `relation`.
    ///
    /// Errors when the start node does not exist in the collection
    /// (REQ-013 FR-008).
    #[allow(clippy::unused_async)]
    async fn neighbors(
        &self,
        ctx: &Context<'_>,
        kind: String,
        key: String,
        relation: Option<String>,
        max_hops: u8,
    ) -> async_graphql::Result<Vec<NeighborOut>> {
        let collection = ctx.data::<CollectionName>()?;
        let id = NodeId::new(parse_kind(&kind)?, key);
        let relation = relation.as_deref().map(parse_relation).transpose()?;
        let guard = self
            .store
            .lock()
            .map_err(|_| async_graphql::Error::new("graph store lock poisoned"))?;
        if guard
            .node(collection, &id)
            .map_err(|error| map_error(&error))?
            .is_none()
        {
            return Err(node_not_found(&id));
        }
        let neighbors = guard
            .neighbors(collection, &id, relation, max_hops)
            .map_err(|error| map_error(&error))?;
        Ok(neighbors.into_iter().map(Into::into).collect())
    }
}

/// Builds the node-not-found query error.
fn node_not_found(id: &NodeId) -> async_graphql::Error {
    async_graphql::Error::new(format!(
        "node not found: {} {}",
        id.kind().as_str(),
        id.key()
    ))
}

impl From<Neighbor> for NeighborOut {
    fn from(neighbor: Neighbor) -> Self {
        let node = neighbor.node();
        Self {
            kind: node.id().kind().as_str().to_owned(),
            key: node.id().key().to_owned(),
            title: node.title().to_owned(),
            relation: neighbor.relation().as_str().to_owned(),
            depth: neighbor.depth(),
        }
    }
}

impl From<kv_domain::GraphNode> for NodeOut {
    fn from(node: kv_domain::GraphNode) -> Self {
        Self {
            kind: node.id().kind().as_str().to_owned(),
            key: node.id().key().to_owned(),
            title: node.title().to_owned(),
        }
    }
}

fn parse_kind(raw: &str) -> Result<EntityKind, async_graphql::Error> {
    EntityKind::from_key(raw)
        .ok_or_else(|| async_graphql::Error::new(format!("unknown node kind: {raw}")))
}

fn parse_relation(raw: &str) -> Result<RelationKind, async_graphql::Error> {
    RelationKind::from_key(raw)
        .ok_or_else(|| async_graphql::Error::new(format!("unknown relation: {raw}")))
}

fn map_error(error: &GraphStoreError) -> async_graphql::Error {
    async_graphql::Error::new(error.to_string())
}

/// Builds the in-process graph query schema over the given store handle.
pub fn build_schema<S: GraphStore + Send + 'static>(
    store: S,
    collection: CollectionName,
) -> Schema<GraphQueryRoot<S>, EmptyMutation, EmptySubscription> {
    Schema::build(
        GraphQueryRoot {
            store: Mutex::new(store),
        },
        EmptyMutation,
        EmptySubscription,
    )
    .data(collection)
    .finish()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use kv_application::{CollectionStore, FileRecord, FileStore};
    use kv_domain::{CollectionName, Timestamp};
    use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore, SqliteGraphStore};
    use tempfile::tempdir;

    fn graphql_query(
        store: SqliteGraphStore,
        doc: &str,
    ) -> Result<async_graphql::Response, Box<dyn Error>> {
        let schema = build_schema(store, CollectionName::try_from("Notes")?);
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        let response = runtime.block_on(schema.execute(doc));
        Ok(response)
    }

    fn build_store()
    -> Result<(SqliteGraphStore, CollectionName, std::path::PathBuf), Box<dyn Error>> {
        let directory = tempdir()?;
        let database_path = directory.path().join("collections.db");
        let collection = CollectionName::try_from("Notes")?;
        let mut collections = SqliteCollectionStore::open(&database_path)?;
        collections.create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;

        let a = directory.path().join("a.md");
        let b = directory.path().join("b.md");
        let mut store = SqliteFileStore::open_for_ingestion(&database_path)?;
        store.upsert_files(
            &collection,
            &[
                FileRecord::new(a.clone(), b"---\n---\n[to](b.md)\n".to_vec()),
                FileRecord::new(b, b"---\n---\nbody\n".to_vec()),
            ],
            Timestamp::from_unix_seconds(1_700_000_000),
        )?;
        store.reconcile(
            &collection,
            &[],
            &[],
            Timestamp::from_unix_seconds(1_700_000_001),
        )?;

        let graph_store = SqliteGraphStore::open(&database_path)?;
        Ok((graph_store, collection, a))
    }

    #[test]
    fn neighbors_query_returns_results() -> Result<(), Box<dyn Error>> {
        let (store, _collection, a) = build_store()?;
        let doc = format!(
            r#"{{ neighbors(kind: "file", key: "{}", maxHops: 2) {{ key relation depth }} }}"#,
            a.to_string_lossy()
        );
        let response = graphql_query(store, &doc)?;
        assert!(response.errors.is_empty(), "{:?}", response.errors);
        Ok(())
    }

    /// Covers: REQ-013 FR-008 — an unknown node query reports an error.
    #[test]
    fn node_query_reports_unknown_node() -> Result<(), Box<dyn Error>> {
        let (store, _collection, _) = build_store()?;
        let doc = r#"{ node(kind: "file", key: "zzz.md") { key } }"#;
        let response = graphql_query(store, doc)?;
        assert!(
            !response.errors.is_empty(),
            "expected a node-not-found error"
        );
        Ok(())
    }
}
