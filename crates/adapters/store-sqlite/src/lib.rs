#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `SQLite` adapter for the mdsearch application ports.

mod collections;
mod files;
mod graph;
mod graph_build;
mod hybrid;
mod inspection;
mod lexical;
mod lexical_build;
mod retrieval;
mod schema;
mod semantic;
mod semantic_commit;
mod storage_errors;

pub use collections::SqliteCollectionStore;
pub use files::SqliteFileStore;
pub(crate) use files::{fingerprint_for_collection, resolve_collection_id, upsert_file};
pub use graph::SqliteGraphStore;
pub(crate) use graph_build::rebuild_graph;
pub use hybrid::SqliteHybridSearchStore;
pub use inspection::SqliteIndexInspectionStore;
pub(crate) use lexical::compute_position;
pub use lexical::{SqliteLexicalIndexStore, SqliteLexicalSearchStore};
pub(crate) use lexical_build::rebuild_index;
pub use retrieval::SqliteFileRetrievalStore;
pub(crate) use schema::{
    INDEX_SCHEMA_VERSION, LEGACY_EMBEDDING_DIMENSION, migrate, register_vector_extension,
    schema_version,
};
pub use semantic::SqliteSemanticIndexStore;
pub(crate) use semantic::vector_blob;
pub(crate) use semantic_commit::{
    configure_global_vector_table, reconcile_files_and_indexes, reconcile_semantic_update,
    replace_collection_semantic, store_global_models,
};
pub(crate) use storage_errors::{
    database_unavailable, file_storage_failure, index_storage_failure, retrieval_storage_failure,
    search_storage_failure, semantic_storage_failure, storage_failure,
};
