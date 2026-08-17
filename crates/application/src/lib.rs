#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `mdsearch`.

mod add_files;
mod create_collection;
mod destroy_collection;
mod error;
mod index_status;
mod lexical_search;
mod list_collections;
mod ports;
mod update_collection;

pub use add_files::{AddFiles, AddFilesOutcome};
pub use create_collection::CreateCollection;
pub use destroy_collection::DestroyCollection;
pub use error::{
    AddFilesError, ClockError, CollectionStoreError, CreateCollectionError, DestroyCollectionError,
    FileStoreError, FileSystemError, IndexStatusError, IndexStoreError, ListCollectionsError,
    SearchError, SearchStoreError, UpdateCollectionError,
};
pub use index_status::ReadIndexStatus;
pub use lexical_search::SearchLexical;
pub use list_collections::ListCollections;
pub use ports::{
    Clock, CollectionStore, FileRecord, FileStore, FileSystem, IndexState, IndexStatus,
    LexicalIndexStore, LexicalSearchStore, Position, ReconcileOutcome, SearchResult,
    SearchResultSet, SearchScope, StoredFile,
};
pub use update_collection::{UpdateCollection, UpdateOutcome, UpdateTarget};
