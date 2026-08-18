#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `mdsearch`.

mod add_files;
mod create_collection;
mod destroy_collection;
mod embed_collections;
mod error;
mod get_file;
mod index_status;
mod lexical_search;
mod list_collections;
mod ports;
mod update_collection;

pub use add_files::{AddFiles, AddFilesOutcome};
pub use create_collection::CreateCollection;
pub use destroy_collection::DestroyCollection;
pub use embed_collections::{EmbedCollections, EmbedOutcome, EmbedReport, EmbedScope, SkipReason};
pub use error::{
    AddFilesError, ClockError, CollectionStoreError, CreateCollectionError, DestroyCollectionError,
    EmbedError, EmbeddingError, FileRetrievalStoreError, FileStoreError, FileSystemError,
    GetFileError, IndexStatusError, IndexStoreError, ListCollectionsError, SearchError,
    SearchStoreError, SemanticIndexStoreError, UpdateCollectionError,
};
pub use get_file::GetFile;
pub use index_status::ReadIndexStatus;
pub use lexical_search::SearchLexical;
pub use list_collections::ListCollections;
pub use ports::{
    Clock, CollectionStore, EmbedTarget, EmbeddingGenerator, FileRecord, FileRetrievalStore,
    FileStore, FileSystem, IndexState, IndexStatus, LexicalIndexStore, LexicalSearchStore,
    Position, ReconcileOutcome, RetrievedFile, SearchResult, SearchResultSet, SearchScope,
    SemanticIndexStore, StoredFile,
};
pub use update_collection::{UpdateCollection, UpdateOutcome, UpdateTarget};
