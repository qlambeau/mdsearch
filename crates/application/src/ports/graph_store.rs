use std::collections::{BTreeSet, HashMap, HashSet};

use kv_domain::{CollectionName, EntityGraph, GraphNode, NodeId, RelationKind};

use crate::GraphStoreError;

/// A neighbor of a node reached during graph traversal.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Neighbor {
    node: GraphNode,
    relation: RelationKind,
    depth: u8,
}

impl Neighbor {
    /// Creates a neighbor record from its node, the relation of the reaching
    /// edge, and the traversal depth.
    #[must_use]
    pub fn new(node: GraphNode, relation: RelationKind, depth: u8) -> Self {
        Self {
            node,
            relation,
            depth,
        }
    }

    /// Returns the neighbor node.
    #[must_use]
    pub const fn node(&self) -> &GraphNode {
        &self.node
    }

    /// Returns the relation of the edge that reached this neighbor.
    #[must_use]
    pub const fn relation(&self) -> RelationKind {
        self.relation
    }

    /// Returns the traversal depth at which this neighbor was reached.
    #[must_use]
    pub const fn depth(&self) -> u8 {
        self.depth
    }
}

/// Reads an entity graph built by `mdsearch update`.
///
/// The store is read-only: it never mutates the graph or any other stored
/// state. Implementations must honor the documented lookup, ordering, depth,
/// cycle, and error contracts.
pub trait GraphStore {
    /// Returns the node with the given identity, if it exists in the collection.
    ///
    /// # Errors
    ///
    /// Returns [`GraphStoreError::CollectionNotFound`] when the collection is
    /// unknown, [`GraphStoreError::DatabaseNotFound`] when the database is
    /// missing, and [`GraphStoreError::Storage`] for other failures.
    fn node(
        &self,
        collection: &CollectionName,
        id: &NodeId,
    ) -> Result<Option<GraphNode>, GraphStoreError>;

    /// Returns the neighbors of `id` reachable within `max_hops` hops.
    ///
    /// When `relation` is `Some`, only edges of that relation are traversed. The
    /// traversal is breadth-first with a cycle guard, so it terminates and never
    /// revisits a node. Results are ordered by depth then node identity.
    ///
    /// # Errors
    ///
    /// Returns [`GraphStoreError::CollectionNotFound`] when the collection is
    /// unknown, [`GraphStoreError::DatabaseNotFound`] when the database is
    /// missing, and [`GraphStoreError::Storage`] for other failures.
    fn neighbors(
        &self,
        collection: &CollectionName,
        id: &NodeId,
        relation: Option<RelationKind>,
        max_hops: u8,
    ) -> Result<Vec<Neighbor>, GraphStoreError>;
}

/// Performs a breadth-first, cycle-guarded traversal over `graph` starting at
/// `start`, optionally filtered by `relation`, up to `max_hops` deep.
///
/// Shared by the in-memory fake and any non-CTE implementation so traversal
/// semantics stay identical across stores.
#[must_use]
pub fn traverse_graph(
    graph: &EntityGraph,
    start: &NodeId,
    relation: Option<RelationKind>,
    max_hops: u8,
) -> Vec<Neighbor> {
    let mut results: BTreeSet<Neighbor> = BTreeSet::new();
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut frontier: Vec<(NodeId, u8)> = vec![(start.clone(), 0)];

    while let Some((current, depth)) = frontier.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if depth >= max_hops {
            continue;
        }
        for edge in graph.edges() {
            if edge.src() != &current {
                continue;
            }
            if relation.is_some_and(|want| want != edge.relation()) {
                continue;
            }
            let dst = edge.dst();
            let Some(node) = graph.node(dst) else {
                continue;
            };
            let neighbor = Neighbor::new(node.clone(), edge.relation(), depth + 1);
            results.insert(neighbor.clone());
            frontier.push((dst.clone(), depth + 1));
        }
    }

    results.into_iter().collect()
}

/// An in-memory [`GraphStore`] for tests and as a reference implementation.
///
/// Honors the same contract as production stores over the graphs it holds.
#[derive(Default)]
pub struct InMemoryGraphStore {
    graphs: HashMap<CollectionName, EntityGraph>,
}

impl InMemoryGraphStore {
    /// Creates an empty in-memory graph store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces the graph for a collection.
    pub fn insert(&mut self, collection: CollectionName, graph: EntityGraph) {
        self.graphs.insert(collection, graph);
    }

    fn graph(&self, collection: &CollectionName) -> Result<&EntityGraph, GraphStoreError> {
        self.graphs
            .get(collection)
            .ok_or(GraphStoreError::CollectionNotFound)
    }
}

impl GraphStore for InMemoryGraphStore {
    fn node(
        &self,
        collection: &CollectionName,
        id: &NodeId,
    ) -> Result<Option<GraphNode>, GraphStoreError> {
        let graph = self.graph(collection)?;
        Ok(graph.nodes().find(|node| node.id() == id).cloned())
    }

    fn neighbors(
        &self,
        collection: &CollectionName,
        id: &NodeId,
        relation: Option<RelationKind>,
        max_hops: u8,
    ) -> Result<Vec<Neighbor>, GraphStoreError> {
        let graph = self.graph(collection)?;
        Ok(traverse_graph(graph, id, relation, max_hops))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use kv_domain::{
        CollectionName, EntityGraph, EntityKind, FileId, GraphEdge, GraphSource, NodeId,
        RelationKind, extract_graph,
    };

    use super::*;

    fn node_id(kind: EntityKind, key: &str) -> NodeId {
        NodeId::new(kind, key.to_owned())
    }

    fn build_graph() -> Result<(CollectionName, EntityGraph), Box<dyn Error>> {
        let name = CollectionName::try_from("notes")?;
        let files = [
            GraphSource::new(
                FileId::try_new(1)?,
                std::path::Path::new("a.md"),
                b"---\n---\n[to](b.md)\n",
            ),
            GraphSource::new(
                FileId::try_new(2)?,
                std::path::Path::new("b.md"),
                b"---\n---\n[to](c.md)\n",
            ),
            GraphSource::new(
                FileId::try_new(3)?,
                std::path::Path::new("c.md"),
                b"---\n---\nbody\n",
            ),
        ];
        Ok((name, extract_graph(&files)))
    }

    #[test]
    fn node_lookup_finds_and_reports_missing() -> Result<(), Box<dyn Error>> {
        let (name, graph) = build_graph()?;
        let mut store = InMemoryGraphStore::new();
        store.insert(name.clone(), graph);

        let found = store.node(&name, &node_id(EntityKind::File, "a.md"))?;
        assert!(found.is_some());

        let missing = store.node(&name, &node_id(EntityKind::File, "zzz.md"))?;
        assert!(missing.is_none());

        Ok(())
    }

    #[test]
    fn neighbors_filter_by_relation() -> Result<(), Box<dyn Error>> {
        let (name, graph) = build_graph()?;
        let mut store = InMemoryGraphStore::new();
        store.insert(name.clone(), graph);

        let neighbors = store.neighbors(
            &name,
            &node_id(EntityKind::File, "a.md"),
            Some(RelationKind::LinksTo),
            1,
        )?;
        assert_eq!(neighbors.len(), 1);
        let first = neighbors.first().ok_or("expected one neighbor")?;
        assert_eq!(first.node().id().key(), "b.md");
        assert_eq!(first.relation(), RelationKind::LinksTo);
        assert_eq!(first.depth(), 1);

        Ok(())
    }

    #[test]
    fn traversal_stops_at_hop_limit_and_is_cycle_safe() -> Result<(), Box<dyn Error>> {
        let (name, graph) = build_graph()?;
        let mut store = InMemoryGraphStore::new();
        store.insert(name.clone(), graph);

        let one_hop = store.neighbors(&name, &node_id(EntityKind::File, "a.md"), None, 1)?;
        let keys: Vec<&str> = one_hop.iter().map(|n| n.node().id().key()).collect();
        assert_eq!(keys, vec!["b.md"]);

        let two_hops = store.neighbors(&name, &node_id(EntityKind::File, "a.md"), None, 2)?;
        let keys: Vec<&str> = two_hops.iter().map(|n| n.node().id().key()).collect();
        assert_eq!(keys, vec!["b.md", "c.md"]);

        Ok(())
    }

    #[test]
    fn unknown_collection_is_an_error() -> Result<(), Box<dyn Error>> {
        let store = InMemoryGraphStore::new();
        let missing = CollectionName::try_from("ghost")?;
        let result = store.node(&missing, &node_id(EntityKind::File, "a.md"));
        assert!(matches!(result, Err(GraphStoreError::CollectionNotFound)));

        Ok(())
    }

    #[test]
    fn edge_type_is_reachable_from_api() {
        let edge = GraphEdge::new(
            node_id(EntityKind::File, "a.md"),
            node_id(EntityKind::File, "b.md"),
            RelationKind::LinksTo,
        );
        assert_eq!(edge.relation(), RelationKind::LinksTo);
    }
}
