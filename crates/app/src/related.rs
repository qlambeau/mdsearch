//! File-to-file related context recovery for `search`/`hybrid` results.
//!
//! `related_files` reads the direct file-to-file neighbors of a result file
//! from the entity graph through the [`GraphStore`] port, restricted to the
//! closed relation set (`LINKS_TO`, `RELATED_TO`, `HAS_SOURCE`), omitting tag
//! and alias nodes (REQ-013 FR-001).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kv_application::{GraphStore, GraphStoreError};
use kv_domain::{CollectionName, EntityKind, NodeId, RelationKind};

/// A single file-to-file related link for a result.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelatedFile {
    path: PathBuf,
    relation: RelationKind,
}

impl RelatedFile {
    /// Creates a related link from its file path and relation.
    #[must_use]
    pub fn new(path: PathBuf, relation: RelationKind) -> Self {
        Self { path, relation }
    }

    /// Returns the related file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the relation of the link.
    #[must_use]
    pub const fn relation(&self) -> RelationKind {
        self.relation
    }
}

/// Returns the direct file-to-file related links of the file at `path` in
/// `collection`.
///
/// Tag and alias nodes are omitted, only the closed relation set is kept,
/// results are deduplicated and deterministically ordered, and a missing node
/// or graph yields an empty set rather than an error (REQ-013 FR-001, FR-012).
#[must_use]
pub fn related_files(
    store: &dyn GraphStore,
    collection: &CollectionName,
    path: &Path,
) -> Vec<RelatedFile> {
    let id = NodeId::new(EntityKind::File, path.to_string_lossy().into_owned());
    let neighbors = match store.neighbors(collection, &id, None, 1) {
        Ok(neighbors) => neighbors,
        Err(GraphStoreError::CollectionNotFound | GraphStoreError::DatabaseNotFound) => {
            return Vec::new();
        }
        Err(GraphStoreError::Storage(_)) => return Vec::new(),
    };

    let mut seen = BTreeSet::new();
    for neighbor in neighbors {
        if neighbor.node().id().kind() != EntityKind::File {
            continue;
        }
        if !matches!(
            neighbor.relation(),
            RelationKind::LinksTo | RelationKind::RelatedTo | RelationKind::HasSource
        ) {
            continue;
        }
        seen.insert((neighbor.node().id().key().to_owned(), neighbor.relation()));
    }

    seen.into_iter()
        .map(|(key, relation)| RelatedFile::new(PathBuf::from(key), relation))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use kv_application::InMemoryGraphStore;
    use kv_domain::{EntityGraph, GraphSource, extract_graph};

    use super::*;

    fn build_store() -> Result<(CollectionName, InMemoryGraphStore), Box<dyn Error>> {
        let name = CollectionName::try_from("Notes")?;
        let files = [
            GraphSource::new(
                kv_domain::FileId::try_new(1)?,
                Path::new("a.md"),
                b"---\ntags: [rust]\n---\n[to](b.md)\n[c](c.md)\n",
            ),
            GraphSource::new(
                kv_domain::FileId::try_new(2)?,
                Path::new("b.md"),
                b"---\nrelated: [a]\n---\nbody\n",
            ),
            GraphSource::new(
                kv_domain::FileId::try_new(3)?,
                Path::new("c.md"),
                b"---\nsources: [a]\n---\nbody\n",
            ),
        ];
        let graph: EntityGraph = extract_graph(&files);
        let mut store = InMemoryGraphStore::new();
        store.insert(name.clone(), graph);
        Ok((name, store))
    }

    /// Covers: REQ-013 FR-001 — only file-to-file relations are kept.
    #[test]
    fn keeps_only_file_to_file_relations() -> Result<(), Box<dyn Error>> {
        let (name, store) = build_store()?;
        let related = related_files(&store, &name, Path::new("a.md"));
        let relations: Vec<RelationKind> = related.iter().map(RelatedFile::relation).collect();
        assert!(relations.iter().all(|relation| matches!(
            relation,
            RelationKind::LinksTo | RelationKind::RelatedTo | RelationKind::HasSource
        )));
        let paths: Vec<String> = related
            .iter()
            .map(|file| file.path().to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path == "b.md"));
        assert!(paths.iter().any(|path| path == "c.md"));
        Ok(())
    }

    /// Covers: REQ-013 FR-001 — tag and alias nodes are never listed.
    #[test]
    fn omits_tags_and_aliases() -> Result<(), Box<dyn Error>> {
        let (name, store) = build_store()?;
        let related = related_files(&store, &name, Path::new("a.md"));
        for file in &related {
            assert_eq!(file.path().extension(), Some(std::ffi::OsStr::new("md")));
        }
        Ok(())
    }

    /// Covers: REQ-013 FR-012 — a missing node yields an empty set, no error.
    #[test]
    fn missing_node_yields_empty_set() -> Result<(), Box<dyn Error>> {
        let (name, store) = build_store()?;
        let related = related_files(&store, &name, Path::new("zzz.md"));
        assert!(related.is_empty());
        Ok(())
    }

    /// Covers: REQ-013 FR-012 — a missing collection yields an empty set.
    #[test]
    fn missing_collection_yields_empty_set() -> Result<(), Box<dyn Error>> {
        let (_, store) = build_store()?;
        let ghost = CollectionName::try_from("ghost")?;
        let related = related_files(&store, &ghost, Path::new("a.md"));
        assert!(related.is_empty());
        Ok(())
    }

    /// Covers: REQ-013 FR-001 — results are deduplicated and deterministic.
    #[test]
    fn deduplicates_and_is_deterministic() -> Result<(), Box<dyn Error>> {
        let name = CollectionName::try_from("Notes")?;
        let files = [
            GraphSource::new(
                kv_domain::FileId::try_new(1)?,
                Path::new("a.md"),
                b"---\n---\n[to](b.md)\n[to](b.md)\n",
            ),
            GraphSource::new(
                kv_domain::FileId::try_new(2)?,
                Path::new("b.md"),
                b"---\n---\nbody\n",
            ),
        ];
        let graph = extract_graph(&files);
        let mut store = InMemoryGraphStore::new();
        store.insert(name.clone(), graph);

        let first = related_files(&store, &name, Path::new("a.md"));
        let second = related_files(&store, &name, Path::new("a.md"));
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        Ok(())
    }
}
