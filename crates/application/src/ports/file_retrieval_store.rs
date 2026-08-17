use std::path::{Path, PathBuf};

use kv_domain::{CollectionName, FileId};

use crate::FileRetrievalStoreError;

/// A stored file's canonical path and content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievedFile {
    path: PathBuf,
    content: Vec<u8>,
}

impl RetrievedFile {
    /// Creates a retrieved-file record.
    #[must_use]
    pub const fn new(path: PathBuf, content: Vec<u8>) -> Self {
        Self { path, content }
    }

    /// Returns the canonical path of the file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the stored content bytes.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

/// Looks up stored files for retrieval.
pub trait FileRetrievalStore {
    /// Returns the file at exactly `path` in `collection`, if present.
    ///
    /// # Errors
    ///
    /// Returns a not-found or storage error when the lookup cannot complete.
    fn get_by_path(
        &self,
        collection: &CollectionName,
        path: &Path,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError>;

    /// Returns the file with `id` in `collection`, if present.
    ///
    /// # Errors
    ///
    /// Returns a not-found or storage error when the lookup cannot complete.
    fn get_by_id(
        &self,
        collection: &CollectionName,
        id: FileId,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError>;

    /// Returns every file in `collection` whose basename equals `basename`.
    ///
    /// # Errors
    ///
    /// Returns a not-found or storage error when the lookup cannot complete.
    fn list_by_basename(
        &self,
        collection: &CollectionName,
        basename: &str,
    ) -> Result<Vec<RetrievedFile>, FileRetrievalStoreError>;
}
