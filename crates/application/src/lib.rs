#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `mdsearch`.

mod add_files;
mod create_collection;
mod destroy_collection;
mod embed_collections;
mod error;
mod get_file;
mod hybrid_search;
mod index_status;
mod lexical_search;
mod list_collections;
mod ports;
mod semantic_update;
mod update_collection;

pub use add_files::{AddFiles, AddFilesOutcome};
pub use create_collection::CreateCollection;
pub use destroy_collection::DestroyCollection;
pub use embed_collections::{
    EmbedCollections, EmbedOutcome, EmbedProgress, EmbedReport, EmbedScope, SkipReason,
};
pub use error::{
    AddFilesError, ClockError, CollectionStoreError, CreateCollectionError, DestroyCollectionError,
    EmbedError, EmbeddingError, FileRetrievalStoreError, FileStoreError, FileSystemError,
    GetFileError, GraphStoreError, HybridError, HybridSearchStoreError, IndexStatusError,
    IndexStoreError, ListCollectionsError, RerankError, SearchError, SearchStoreError,
    SemanticIndexStoreError, UpdateCollectionError,
};
pub use get_file::{FileSelector, GetFile};
pub use hybrid_search::{HybridResult, HybridResultSet, HybridSearch};
pub use index_status::ReadIndexStatus;
pub use lexical_search::SearchLexical;
pub use list_collections::ListCollections;
#[cfg(test)]
pub use ports::collection_source_store_fake;
pub use ports::{
    Clock, CollectionIndexUpdate, CollectionSourceStore, CollectionSourceSummary, CollectionStore,
    EmbedTarget, EmbeddingGenerator, FileRecord, FileRetrievalStore, FileStore, FileSystem,
    GraphStore, HybridCandidate, HybridCandidates, HybridSearchStore, InMemoryGraphStore,
    IndexState, IndexStatus, LexicalIndexStore, LexicalSearchStore, ModelAvailability, Neighbor,
    Position, PreparedSemanticCollection, PreparedSemanticPassage, ReconcileOutcome, Reranker,
    RetrievedFile, SearchFile, SearchResult, SearchResultSet, SearchScope, SemanticIndexStore,
    SemanticStatus, StoredFile, traverse_graph,
};
pub use semantic_update::{SemanticUpdateCoordinator, SemanticUpdatePorts};
pub use update_collection::{UpdateCollection, UpdateOutcome, UpdateTarget};

pub use ports::{
    CollectionIndexInspection, DatabaseIndexInspection, IndexInspection, IndexInspectionStore,
};
