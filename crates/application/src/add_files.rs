use std::path::PathBuf;

use kv_domain::{CollectionName, Timestamp};

use crate::{AddFilesError, Clock, FileRecord, FileStore, FileSystem};

/// The outcome of an add-files operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddFilesOutcome {
    added: usize,
    skipped: usize,
}

impl AddFilesOutcome {
    /// Creates an outcome with the given added and skipped counts.
    #[must_use]
    pub const fn new(added: usize, skipped: usize) -> Self {
        Self { added, skipped }
    }

    /// Returns the number of files added.
    #[must_use]
    pub const fn added(&self) -> usize {
        self.added
    }

    /// Returns the number of unreadable paths skipped.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }
}

/// Orchestrates adding markdown files to a collection.
pub struct AddFiles<FS, S, C> {
    filesystem: FS,
    files: S,
    clock: C,
}

impl<FS, S, C> AddFiles<FS, S, C>
where
    FS: FileSystem,
    S: FileStore,
    C: Clock,
{
    /// Creates an add-files use case with its filesystem, store, and clock
    /// ports.
    #[must_use]
    pub const fn new(filesystem: FS, files: S, clock: C) -> Self {
        Self {
            filesystem,
            files,
            clock,
        }
    }

    /// Adds the markdown files reachable from `paths` to `collection`.
    ///
    /// # Errors
    ///
    /// Returns a filesystem, file-store, or clock error when ingestion cannot
    /// complete.
    pub fn execute(
        &mut self,
        collection: &CollectionName,
        paths: &[PathBuf],
        force: bool,
    ) -> Result<AddFilesOutcome, AddFilesError> {
        let mut discovered = Vec::new();
        let mut skipped = 0;

        for path in paths {
            match self.filesystem.expand(path) {
                Ok(files) => discovered.extend(files),
                Err(_) if force => skipped += 1,
                Err(error) => return Err(error.into()),
            }
        }

        let mut records = Vec::new();
        for path in discovered {
            match self.filesystem.read(&path) {
                Ok(content) => records.push(FileRecord::new(path, content)),
                Err(_) if force => skipped += 1,
                Err(error) => return Err(error.into()),
            }
        }

        let ingested_at: Timestamp = self.clock.now()?;
        self.files.upsert_files(collection, &records, ingested_at)?;

        Ok(AddFilesOutcome::new(records.len(), skipped))
    }
}
