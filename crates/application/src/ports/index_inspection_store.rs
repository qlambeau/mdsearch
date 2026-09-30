use crate::IndexStoreError;
use kv_domain::{
    CollectionName, EmbeddingModel, IndexEnablement, IndexFreshness, RerankerModel, Timestamp,
};

/// Build metadata of one index (REQ-023 FR-022/023).
#[derive(Clone, Debug)]
pub struct IndexInspection {
    /// Configured maintenance policy.
    pub enablement: IndexEnablement,
    /// Last successful build; absent means never built.
    pub built_at: Option<Timestamp>,
    /// Stored-content comparison, absent until a successful build exists.
    pub freshness: Option<IndexFreshness>,
    /// Whether recorded model/dimension matches active configuration.
    pub compatible: bool,
    /// Indexed passage count where applicable.
    pub passage_count: usize,
    /// Indexed graph node count where applicable.
    pub node_count: usize,
    /// Indexed graph edge count where applicable.
    pub edge_count: usize,
    /// Embedding model recorded at build time.
    pub model: Option<EmbeddingModel>,
    /// Vector dimension recorded at build time.
    pub dimension: Option<usize>,
}

/// Inspection of all configured indexes for one collection.
#[derive(Clone, Debug)]
pub struct CollectionIndexInspection {
    /// Retained collection identity.
    pub collection: CollectionName,
    /// Number of stored files.
    pub file_count: usize,
    /// Lexical index inspection.
    pub lexical: IndexInspection,
    /// Graph index inspection.
    pub graph: IndexInspection,
    /// Semantic index inspection, including disabled policy.
    pub semantic: IndexInspection,
}

/// A consistent database inspection with selected global models.
#[derive(Clone, Debug)]
pub struct DatabaseIndexInspection {
    /// Selected global embedding model (including the default).
    pub embedding_model: EmbeddingModel,
    /// Active vector table dimension.
    pub embedding_dimension: usize,
    /// Selected global reranker (including the default).
    pub reranker_model: RerankerModel,
    /// Collection inspections in stable name order.
    pub collections: Vec<CollectionIndexInspection>,
}

/// Reads index readiness and freshness without mutations or source access.
pub trait IndexInspectionStore {
    /// Returns one consistent snapshot, optionally scoped to a collection.
    ///
    /// Reads stored bytes and index state only. Legacy schemas remain unchanged.
    /// No filesystem source scan, model loading, download, or migration occurs.
    /// Results are ordered by canonical collection name.
    ///
    /// # Errors
    /// Returns collection-not-found for an unknown scope, or a storage error.
    fn inspect(
        &self,
        collection: Option<&CollectionName>,
    ) -> Result<DatabaseIndexInspection, IndexStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryIndexInspectionStore(DatabaseIndexInspection);
    impl IndexInspectionStore for InMemoryIndexInspectionStore {
        fn inspect(
            &self,
            collection: Option<&CollectionName>,
        ) -> Result<DatabaseIndexInspection, IndexStoreError> {
            let mut result = self.0.clone();
            if let Some(name) = collection {
                result.collections.retain(|entry| entry.collection == *name);
                if result.collections.is_empty() {
                    return Err(IndexStoreError::CollectionNotFound);
                }
            }
            Ok(result)
        }
    }

    /// Covers: REQ-023 FR-022 — the port double retains model metadata and scope errors.
    #[test]
    fn inspection_fake_preserves_models_and_missing_scope() -> Result<(), Box<dyn std::error::Error>>
    {
        let store = InMemoryIndexInspectionStore(DatabaseIndexInspection {
            embedding_model: EmbeddingModel::try_new("all-MiniLM-L6-v2")?,
            embedding_dimension: 384,
            reranker_model: RerankerModel::try_new("bge-reranker-base")?,
            collections: Vec::new(),
        });

        let all = store.inspect(None)?;

        assert_eq!(all.embedding_dimension, 384);
        assert!(all.collections.is_empty());
        assert!(matches!(
            store.inspect(Some(&CollectionName::try_from("missing")?)),
            Err(IndexStoreError::CollectionNotFound)
        ));
        Ok(())
    }
}
