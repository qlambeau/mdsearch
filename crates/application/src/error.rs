use std::error::Error;
use std::path::PathBuf;

use thiserror::Error;

/// Describes a failure while creating a collection in a store.
#[derive(Debug, Error)]
pub enum CollectionStoreError {
    /// A case-insensitive equivalent already exists.
    #[error("collection name is already in use")]
    Duplicate,
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The selected database does not exist.
    #[error("database does not exist")]
    DatabaseNotFound,
    /// The selected database could not be created or opened.
    #[error("database is unavailable")]
    DatabaseUnavailable(#[source] Box<dyn Error + Send + Sync>),
    /// The database operation failed after it was opened.
    #[error("collection storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure while obtaining the current time.
#[derive(Debug, Error)]
pub enum ClockError {
    /// The system clock could not represent the current time.
    #[error("system clock is unavailable")]
    Unavailable(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the create-collection use case.
#[derive(Debug, Error)]
pub enum CreateCollectionError {
    /// The clock could not provide creation metadata.
    #[error(transparent)]
    Clock(#[from] ClockError),
    /// The collection store rejected or failed the creation.
    #[error(transparent)]
    Store(#[from] CollectionStoreError),
}

/// Describes a failure from the list-collections use case.
#[derive(Debug, Error)]
pub enum ListCollectionsError {
    /// The collection store rejected or failed the listing.
    #[error(transparent)]
    Store(#[from] CollectionStoreError),
}

/// Describes a failure from the destroy-collection use case.
#[derive(Debug, Error)]
pub enum DestroyCollectionError {
    /// The collection store rejected or failed the destruction.
    #[error(transparent)]
    Store(#[from] CollectionStoreError),
}

/// Describes a failure while discovering or reading markdown files.
#[derive(Debug, Error)]
pub enum FileSystemError {
    /// The path does not exist or cannot be read.
    #[error("path is unreadable: {path}")]
    Unreadable {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
}

/// Describes a failure while storing files in a collection.
#[derive(Debug, Error)]
pub enum FileStoreError {
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The database operation failed after it was opened.
    #[error("file storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the add-files use case.
#[derive(Debug, Error)]
pub enum AddFilesError {
    /// The clock could not provide ingest timestamps.
    #[error(transparent)]
    Clock(#[from] ClockError),
    /// A file could not be discovered or read.
    #[error(transparent)]
    FileSystem(#[from] FileSystemError),
    /// The file store rejected or failed the ingestion.
    #[error(transparent)]
    FileStore(#[from] FileStoreError),
}

/// Describes a failure from the update-collection use case.
#[derive(Debug, Error)]
pub enum UpdateCollectionError {
    /// The clock could not provide ingest timestamps.
    #[error(transparent)]
    Clock(#[from] ClockError),
    /// A file could not be discovered or read.
    #[error(transparent)]
    FileSystem(#[from] FileSystemError),
    /// The file store rejected or failed the reconciliation.
    #[error(transparent)]
    FileStore(#[from] FileStoreError),
}

/// Describes a failure while reading lexical index state.
#[derive(Debug, Error)]
pub enum IndexStoreError {
    /// The database operation failed after it was opened.
    #[error("index storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the index-status use case.
#[derive(Debug, Error)]
pub enum IndexStatusError {
    /// The index store rejected or failed the status read.
    #[error(transparent)]
    Store(#[from] IndexStoreError),
}

/// Describes a failure while searching the lexical index.
#[derive(Debug, Error)]
pub enum SearchStoreError {
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The requested collection's index has never been built.
    #[error("lexical index is not built")]
    IndexNotBuilt,
    /// The query is not valid FTS5 match syntax.
    #[error("invalid query: {message}")]
    InvalidQuery {
        /// The engine's description of the query problem.
        message: String,
    },
    /// The database operation failed after it was opened.
    #[error("search storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the lexical-search use case.
#[derive(Debug, Error)]
pub enum SearchError {
    /// The query is empty or whitespace-only.
    #[error("query is empty")]
    EmptyQuery,
    /// The search store rejected or failed the search.
    #[error(transparent)]
    Store(#[from] SearchStoreError),
}

/// Describes a failure while looking up a stored file for retrieval.
#[derive(Debug, Error)]
pub enum FileRetrievalStoreError {
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The database operation failed after it was opened.
    #[error("file retrieval storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the get-file use case.
#[derive(Debug, Error)]
pub enum GetFileError {
    /// No file matches the supplied name or ID.
    #[error("file not found")]
    FileNotFound,
    /// The basename matches more than one file.
    #[error("ambiguous basename; candidate paths: {0:?}")]
    Ambiguous(Vec<std::path::PathBuf>),
    /// The retrieval store rejected or failed the lookup.
    #[error(transparent)]
    Store(#[from] FileRetrievalStoreError),
}

/// Describes a failure from the embedding generator.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    /// The model is not supported by the embedding library.
    #[error("embedding model {model} is not supported")]
    UnsupportedModel {
        /// The model that is not supported.
        model: String,
    },
    /// The model's assets are not cached locally and no download was requested.
    #[error("embedding model {model} is not available locally; pass --download to fetch it")]
    ModelNotCached {
        /// The model whose assets are missing.
        model: String,
    },
    /// The model assets could not be downloaded.
    #[error("embedding model {model} download failed")]
    DownloadFailed {
        /// The model whose assets could not be downloaded.
        model: String,
        /// The underlying download error.
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
    /// Embedding generation failed.
    #[error("embedding failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure while reading or writing the semantic index.
#[derive(Debug, Error)]
pub enum SemanticIndexStoreError {
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The database operation failed after it was opened.
    #[error("semantic index storage failed")]
    Storage(#[source] Box<dyn Error + Send + Sync>),
}

/// Describes a failure from the embed-collections use case.
#[derive(Debug, Error)]
pub enum EmbedError {
    /// The embedding generator rejected or failed the model or embedding.
    #[error(transparent)]
    Generator(#[from] EmbeddingError),
    /// The clock could not provide embed timestamps.
    #[error(transparent)]
    Clock(#[from] ClockError),
    /// The semantic index store rejected or failed the operation.
    #[error(transparent)]
    Store(#[from] SemanticIndexStoreError),
    /// The requested collection does not exist.
    #[error("collection not found")]
    CollectionNotFound,
    /// The requested collection's lexical index has never been built.
    #[error("lexical index is not built")]
    IndexNotBuilt,
}
