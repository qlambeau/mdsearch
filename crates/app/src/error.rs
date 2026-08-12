use thiserror::Error;

use kv_application::{CollectionStoreError, CreateCollectionError};
use kv_domain::CollectionNameError;

/// Describes a user-visible failure from the `kv` CLI.
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
    /// The database could not be opened or initialized.
    #[error(transparent)]
    DatabaseUnavailable(#[from] CollectionStoreError),
}
