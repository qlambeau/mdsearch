//! `SQLite` storage errors persistence.

use std::error::Error;

use kv_application::{
    CollectionStoreError, FileRetrievalStoreError, FileStoreError, IndexStoreError,
    SearchStoreError, SemanticIndexStoreError,
};

pub(super) fn semantic_storage_failure(
    error: impl Error + Send + Sync + 'static,
) -> SemanticIndexStoreError {
    SemanticIndexStoreError::Storage(Box::new(error))
}

pub(super) fn database_unavailable(
    error: impl Error + Send + Sync + 'static,
) -> CollectionStoreError {
    CollectionStoreError::DatabaseUnavailable(Box::new(error))
}

pub(super) fn storage_failure(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::Storage(Box::new(error))
}

pub(super) fn file_storage_failure(error: impl Error + Send + Sync + 'static) -> FileStoreError {
    FileStoreError::Storage(Box::new(error))
}

pub(super) fn index_storage_failure(error: impl Error + Send + Sync + 'static) -> IndexStoreError {
    IndexStoreError::Storage(Box::new(error))
}

pub(super) fn search_storage_failure(
    error: impl Error + Send + Sync + 'static,
) -> SearchStoreError {
    SearchStoreError::Storage(Box::new(error))
}

pub(super) fn retrieval_storage_failure(
    error: impl Error + Send + Sync + 'static,
) -> FileRetrievalStoreError {
    FileRetrievalStoreError::Storage(Box::new(error))
}
