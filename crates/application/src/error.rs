use std::error::Error;

use thiserror::Error;

/// Describes a failure while creating a collection in a store.
#[derive(Debug, Error)]
pub enum CollectionStoreError {
    /// A case-insensitive equivalent already exists.
    #[error("collection name is already in use")]
    Duplicate,
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
