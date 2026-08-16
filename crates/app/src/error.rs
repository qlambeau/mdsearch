use thiserror::Error;

use kv_application::{
    CollectionStoreError, CreateCollectionError, DestroyCollectionError, ListCollectionsError,
};
use kv_domain::CollectionNameError;

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
    /// The database could not be opened or accessed.
    #[error(transparent)]
    CollectionStore(#[from] CollectionStoreError),
}
