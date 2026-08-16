use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use kv_domain::{CollectionName, ContentHash, Timestamp};

use crate::{Clock, FileRecord, FileStore, FileSystem, StoredFile, UpdateCollectionError};

/// The scope of an update operation.
#[derive(Clone, Copy, Debug)]
pub enum UpdateTarget<'a> {
    /// Walk the supplied paths for added and modified files.
    Paths(&'a [PathBuf]),
    /// Re-read every stored file to detect modifications.
    Stored,
}

/// The outcome of an update-collection operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpdateOutcome {
    added: usize,
    modified: usize,
    deleted: usize,
    skipped: usize,
}

impl UpdateOutcome {
    /// Creates an outcome with the given counts.
    #[must_use]
    pub const fn new(added: usize, modified: usize, deleted: usize, skipped: usize) -> Self {
        Self {
            added,
            modified,
            deleted,
            skipped,
        }
    }

    /// Returns the number of files added.
    #[must_use]
    pub const fn added(&self) -> usize {
        self.added
    }

    /// Returns the number of files modified.
    #[must_use]
    pub const fn modified(&self) -> usize {
        self.modified
    }

    /// Returns the number of files deleted.
    #[must_use]
    pub const fn deleted(&self) -> usize {
        self.deleted
    }

    /// Returns the number of unreadable paths skipped.
    #[must_use]
    pub const fn skipped(&self) -> usize {
        self.skipped
    }
}

/// Orchestrates reconciling a collection with the current filesystem state.
pub struct UpdateCollection<FS, S, C> {
    filesystem: FS,
    files: S,
    clock: C,
}

impl<FS, S, C> UpdateCollection<FS, S, C>
where
    FS: FileSystem,
    S: FileStore,
    C: Clock,
{
    /// Creates an update-collection use case with its filesystem, store, and
    /// clock ports.
    #[must_use]
    pub const fn new(filesystem: FS, files: S, clock: C) -> Self {
        Self {
            filesystem,
            files,
            clock,
        }
    }

    /// Reconciles `collection` with the current filesystem state.
    ///
    /// # Errors
    ///
    /// Returns a filesystem, file-store, or clock error when reconciliation
    /// cannot complete.
    pub fn execute(
        &mut self,
        collection: &CollectionName,
        target: UpdateTarget<'_>,
        force: bool,
    ) -> Result<UpdateOutcome, UpdateCollectionError> {
        let stored = self.files.list_files(collection)?;
        let stored_by_path = stored
            .iter()
            .map(|file| (file.path(), file.content_hash()))
            .collect::<HashMap<_, _>>();

        let (on_disk, walk_skipped) = self.collect_on_disk(target, force)?;
        let (mut to_upsert, added, on_disk_modified) = classify_on_disk(&on_disk, &stored_by_path);

        let on_disk_paths = on_disk.iter().map(FileRecord::path).collect::<HashSet<_>>();
        let stored_changes = self.classify_stored(&stored, &on_disk_paths, target, force)?;
        to_upsert.extend(stored_changes.upsert);

        let ingested_at: Timestamp = self.clock.now()?;
        self.files
            .reconcile(collection, &to_upsert, &stored_changes.delete, ingested_at)?;

        Ok(UpdateOutcome::new(
            added,
            on_disk_modified + stored_changes.modified,
            stored_changes.deleted,
            walk_skipped + stored_changes.skipped,
        ))
    }

    fn collect_on_disk(
        &self,
        target: UpdateTarget<'_>,
        force: bool,
    ) -> Result<(Vec<FileRecord>, usize), UpdateCollectionError> {
        let UpdateTarget::Paths(paths) = target else {
            return Ok((Vec::new(), 0));
        };

        let mut discovered = Vec::new();
        let mut skipped = 0;

        for path in paths {
            match self.filesystem.expand(path) {
                Ok(files) => discovered.extend(files),
                Err(_) if force => skipped += 1,
                Err(error) => return Err(error.into()),
            }
        }

        let mut on_disk = Vec::new();
        for path in discovered {
            match self.filesystem.read(&path) {
                Ok(content) => on_disk.push(FileRecord::new(path, content)),
                Err(_) if force => skipped += 1,
                Err(error) => return Err(error.into()),
            }
        }

        Ok((on_disk, skipped))
    }

    fn classify_stored(
        &self,
        stored: &[StoredFile],
        on_disk_paths: &HashSet<&Path>,
        target: UpdateTarget<'_>,
        force: bool,
    ) -> Result<StoredClassification, UpdateCollectionError> {
        let mut upsert = Vec::new();
        let mut delete = Vec::new();
        let mut modified = 0;
        let mut deleted = 0;
        let mut skipped = 0;

        for file in stored {
            if on_disk_paths.contains(file.path()) {
                continue;
            }

            match self.filesystem.exists(file.path()) {
                Ok(false) => {
                    deleted += 1;
                    delete.push(file.path().to_owned());
                }
                Ok(true) => {
                    if matches!(target, UpdateTarget::Stored) {
                        match self.filesystem.read(file.path()) {
                            Ok(content) => {
                                let hash = ContentHash::from_content(&content);
                                if hash != *file.content_hash() {
                                    modified += 1;
                                    upsert.push(FileRecord::new(file.path().to_owned(), content));
                                }
                            }
                            Err(_) if force => skipped += 1,
                            Err(error) => return Err(error.into()),
                        }
                    }
                }
                Err(_) if force => skipped += 1,
                Err(error) => return Err(error.into()),
            }
        }

        Ok(StoredClassification {
            upsert,
            delete,
            modified,
            deleted,
            skipped,
        })
    }
}

/// The reconciliation changes derived from inspecting stored files.
struct StoredClassification {
    upsert: Vec<FileRecord>,
    delete: Vec<PathBuf>,
    modified: usize,
    deleted: usize,
    skipped: usize,
}

fn classify_on_disk(
    on_disk: &[FileRecord],
    stored_by_path: &HashMap<&Path, &ContentHash>,
) -> (Vec<FileRecord>, usize, usize) {
    let mut to_upsert = Vec::new();
    let mut added = 0;
    let mut modified = 0;

    for record in on_disk {
        match stored_by_path.get(record.path()) {
            Some(stored_hash) if *stored_hash == record.content_hash() => {}
            Some(_) => {
                modified += 1;
                to_upsert.push(record.clone());
            }
            None => {
                added += 1;
                to_upsert.push(record.clone());
            }
        }
    }

    (to_upsert, added, modified)
}
