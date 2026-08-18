use thiserror::Error;

use kv_application::{
    AddFilesError, CollectionStoreError, CreateCollectionError, DestroyCollectionError, EmbedError,
    GetFileError, IndexStatusError, ListCollectionsError, SearchError, UpdateCollectionError,
};
use kv_domain::{CollectionNameError, EmbeddingModelError};

/// Describes a user-visible failure from the `mdsearch` CLI.
#[derive(Debug, Error)]
pub enum AppError {
    /// Clap rejected the command-line arguments.
    #[error(transparent)]
    Arguments(#[from] clap::Error),
    /// The collection name is invalid.
    #[error(transparent)]
    InvalidName(#[from] CollectionNameError),
    /// The application use case failed.
    #[error(transparent)]
    CreateCollection(#[from] CreateCollectionError),
    /// The list-collections use case failed.
    #[error(transparent)]
    ListCollections(#[from] ListCollectionsError),
    /// The destroy-collection use case failed.
    #[error(transparent)]
    DestroyCollection(#[from] DestroyCollectionError),
    /// The add-files use case failed.
    #[error(transparent)]
    AddFiles(#[from] AddFilesError),
    /// The update-collection use case failed.
    #[error(transparent)]
    UpdateCollection(#[from] UpdateCollectionError),
    /// The index-status use case failed.
    #[error(transparent)]
    IndexStatus(#[from] IndexStatusError),
    /// The lexical-search use case failed.
    #[error(transparent)]
    Search(#[from] SearchError),
    /// The get-file use case failed.
    #[error(transparent)]
    GetFile(#[from] GetFileError),
    /// The embed-collections use case failed.
    #[error(transparent)]
    Embed(#[from] EmbedError),
    /// The embedding model name is invalid.
    #[error(transparent)]
    InvalidEmbeddingModel(#[from] EmbeddingModelError),
    /// The embed-collections use case completed with per-collection failures.
    #[error("{0}")]
    EmbedPartial(String),
    /// The retrieved file content is not valid UTF-8.
    #[error("file content is not valid UTF-8")]
    NonUtf8Content,
    /// The database could not be opened or accessed.
    #[error(transparent)]
    CollectionStore(#[from] CollectionStoreError),
}
