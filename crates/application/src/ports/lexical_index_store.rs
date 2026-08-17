use kv_domain::{CollectionName, Timestamp};

use crate::IndexStoreError;

/// Whether a collection's lexical index has been built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexState {
    /// The collection's index was built by a successful update.
    Built,
    /// The collection's index has never been built.
    NotBuilt,
}

/// The observable state of one collection's lexical index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexStatus {
    collection: CollectionName,
    file_count: usize,
    passage_count: usize,
    built_at: Option<Timestamp>,
}

impl IndexStatus {
    /// Creates a status record for a collection's lexical index.
    #[must_use]
    pub const fn new(
        collection: CollectionName,
        file_count: usize,
        passage_count: usize,
        built_at: Option<Timestamp>,
    ) -> Self {
        Self {
            collection,
            file_count,
            passage_count,
            built_at,
        }
    }

    /// Returns the collection this status belongs to.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }

    /// Returns the number of stored files in the collection.
    #[must_use]
    pub const fn file_count(&self) -> usize {
        self.file_count
    }

    /// Returns the number of indexed passages.
    #[must_use]
    pub const fn passage_count(&self) -> usize {
        self.passage_count
    }

    /// Returns when the index was last built, if it has been built.
    #[must_use]
    pub const fn built_at(&self) -> Option<Timestamp> {
        self.built_at
    }

    /// Returns whether the index has been built.
    #[must_use]
    pub const fn state(&self) -> IndexState {
        if self.built_at.is_some() {
            IndexState::Built
        } else {
            IndexState::NotBuilt
        }
    }
}

/// Reads the lexical index state of every collection.
pub trait LexicalIndexStore {
    /// Returns the lexical index status of every collection in the database.
    ///
    /// A collection whose index has never been built reports `NotBuilt` with a
    /// zero passage count and no build timestamp, whether or not the database
    /// has been migrated to the schema version that holds index state.
    ///
    /// # Errors
    ///
    /// Returns a storage error when the status cannot be read.
    fn status(&self) -> Result<Vec<IndexStatus>, IndexStoreError>;
}
