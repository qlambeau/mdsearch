//! Acceptance tests for the update-collection application use case.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use kv_application::{
    Clock, ClockError, FileRecord, FileStore, FileStoreError, FileSystem, FileSystemError,
    StoredFile, UpdateCollection, UpdateTarget,
};
use kv_domain::{CollectionName, Timestamp};

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> Result<Timestamp, ClockError> {
        Ok(Timestamp::from_unix_seconds(1_700_000_000))
    }
}

#[derive(Default)]
struct InMemoryFileSystem {
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl InMemoryFileSystem {
    fn insert(&mut self, path: &str, content: &[u8]) {
        self.files.insert(PathBuf::from(path), content.to_vec());
    }
}

impl FileSystem for InMemoryFileSystem {
    fn expand(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        if !self.files.contains_key(path) {
            return Err(FileSystemError::Unreadable {
                path: path.to_owned(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            });
        }
        Ok(vec![path.to_owned()])
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| FileSystemError::Unreadable {
                path: path.to_owned(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
            })
    }

    fn exists(&self, path: &Path) -> Result<bool, FileSystemError> {
        Ok(self.files.contains_key(path))
    }
}

#[derive(Default)]
struct InMemoryFileStore {
    collections: BTreeMap<String, Vec<FileRecord>>,
}

impl InMemoryFileStore {
    fn add_collection(&mut self, collection: &CollectionName) {
        self.collections
            .entry(collection.name_key().to_owned())
            .or_default();
    }

    fn store_file(&mut self, collection: &CollectionName, path: &str, content: &[u8]) {
        self.collections
            .entry(collection.name_key().to_owned())
            .or_default()
            .push(FileRecord::new(PathBuf::from(path), content.to_vec()));
    }
}

impl FileStore for InMemoryFileStore {
    fn upsert_files(
        &mut self,
        collection: &CollectionName,
        files: &[FileRecord],
        _ingested_at: Timestamp,
    ) -> Result<(), FileStoreError> {
        let stored = self
            .collections
            .get_mut(collection.name_key())
            .ok_or(FileStoreError::CollectionNotFound)?;
        stored.clear();
        stored.extend(files.iter().cloned());
        Ok(())
    }

    fn list_files(&self, collection: &CollectionName) -> Result<Vec<StoredFile>, FileStoreError> {
        let stored = self
            .collections
            .get(collection.name_key())
            .ok_or(FileStoreError::CollectionNotFound)?;
        Ok(stored
            .iter()
            .map(|record| StoredFile::new(record.path().to_owned(), record.content_hash().clone()))
            .collect())
    }

    fn reconcile(
        &mut self,
        collection: &CollectionName,
        upsert: &[FileRecord],
        delete: &[PathBuf],
        _ingested_at: Timestamp,
    ) -> Result<(), FileStoreError> {
        let stored = self
            .collections
            .get_mut(collection.name_key())
            .ok_or(FileStoreError::CollectionNotFound)?;

        for path in delete {
            stored.retain(|record| record.path() != path.as_path());
        }

        for record in upsert {
            match stored
                .iter_mut()
                .find(|existing| existing.path() == record.path())
            {
                Some(existing) => *existing = record.clone(),
                None => stored.push(record.clone()),
            }
        }

        Ok(())
    }
}

fn collection() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

/// Covers: FR-006 — a new on-disk file is added.
#[test]
fn adds_new_files() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("b.md", b"beta");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        UpdateTarget::Paths(&[PathBuf::from("b.md")]),
        false,
    )?;

    assert_eq!(outcome.added(), 1);
    assert_eq!(outcome.modified(), 0);
    assert_eq!(outcome.deleted(), 0);

    Ok(())
}

/// Covers: FR-007 — a changed file is re-ingested.
#[test]
fn modifies_changed_files() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("a.md", b"new content");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    store.store_file(&collection, "a.md", b"old content");
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        UpdateTarget::Paths(&[PathBuf::from("a.md")]),
        false,
    )?;

    assert_eq!(outcome.modified(), 1);

    Ok(())
}

/// Covers: FR-008 — a vanished file is removed.
#[test]
fn removes_deleted_files() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let filesystem = InMemoryFileSystem::default();
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    store.store_file(&collection, "a.md", b"content");
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(&collection, UpdateTarget::Paths(&[]), false)?;

    assert_eq!(outcome.deleted(), 1);

    Ok(())
}

/// Covers: FR-009 — an unchanged file is left as-is.
#[test]
fn leaves_unchanged_files_alone() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("a.md", b"same");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    store.store_file(&collection, "a.md", b"same");
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        UpdateTarget::Paths(&[PathBuf::from("a.md")]),
        false,
    )?;

    assert_eq!(outcome.added(), 0);
    assert_eq!(outcome.modified(), 0);
    assert_eq!(outcome.deleted(), 0);

    Ok(())
}

/// Covers: FR-010 — `--all` (Stored) re-hashes stored files to detect edits.
#[test]
fn detects_modifications_for_the_stored_target() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("a.md", b"edited");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    store.store_file(&collection, "a.md", b"original");
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(&collection, UpdateTarget::Stored, false)?;

    assert_eq!(outcome.modified(), 1);

    Ok(())
}

/// Covers: FR-011 — an unreadable path fails the whole command.
#[test]
fn fails_when_a_path_is_unreadable() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let filesystem = InMemoryFileSystem::default();
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let error = use_case
        .execute(
            &collection,
            UpdateTarget::Paths(&[PathBuf::from("missing.md")]),
            false,
        )
        .err()
        .ok_or_else(|| std::io::Error::other("an unreadable path should fail"))?;

    assert!(matches!(
        error,
        kv_application::UpdateCollectionError::FileSystem(FileSystemError::Unreadable { .. })
    ));

    Ok(())
}

/// Covers: FR-012 — `--force` skips unreadable paths.
#[test]
fn skips_unreadable_paths_when_forced() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("b.md", b"beta");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = UpdateCollection::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        UpdateTarget::Paths(&[PathBuf::from("missing.md"), PathBuf::from("b.md")]),
        true,
    )?;

    assert_eq!(outcome.added(), 1);
    assert_eq!(outcome.skipped(), 1);

    Ok(())
}
