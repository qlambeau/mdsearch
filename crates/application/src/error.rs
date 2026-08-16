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
