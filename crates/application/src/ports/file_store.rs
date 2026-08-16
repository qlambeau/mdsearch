use std::path::{Path, PathBuf};

use kv_domain::{CollectionName, ContentHash, Timestamp};

use crate::FileStoreError;

/// A discovered markdown file ready to be stored.
#[derive(Clone, Debug)]
pub struct FileRecord {
    path: PathBuf,
    content: Vec<u8>,
    content_hash: ContentHash,
}

impl FileRecord {
    /// Creates a file record for the given path and content, hashing the
    /// content.
    #[must_use]
    pub fn new(path: PathBuf, content: Vec<u8>) -> Self {
        let content_hash = ContentHash::from_content(&content);
        Self {
            path,
            content,
            content_hash,
        }
    }

    /// Returns the canonical path of the file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file content.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    /// Returns the content hash.
    #[must_use]
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
}

/// A stored file's identity and content hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredFile {
    path: PathBuf,
    content_hash: ContentHash,
}

impl StoredFile {
    /// Creates a stored-file summary.
    #[must_use]
    pub const fn new(path: PathBuf, content_hash: ContentHash) -> Self {
        Self { path, content_hash }
    }

    /// Returns the canonical path of the stored file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the stored content hash.
    #[must_use]
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
}

/// Stores ingested files for a collection.
pub trait FileStore {
    /// Upserts the given files for the named collection in one transaction.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when persistence cannot complete.
    fn upsert_files(
        &mut self,
        collection: &CollectionName,
        files: &[FileRecord],
        ingested_at: Timestamp,
    ) -> Result<(), FileStoreError>;

    /// Returns the stored files for the named collection.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when the files cannot be read.
    fn list_files(&self, collection: &CollectionName) -> Result<Vec<StoredFile>, FileStoreError>;

    /// Reconciles a collection by upserting `upsert` and deleting `delete` in
    /// one transaction.
    ///
    /// # Errors
    ///
    /// Returns a not-found error when the collection does not exist, or a
    /// storage error when persistence cannot complete.
    fn reconcile(
        &mut self,
        collection: &CollectionName,
        upsert: &[FileRecord],
        delete: &[PathBuf],
        ingested_at: Timestamp,
    ) -> Result<(), FileStoreError>;
}
