//! Acceptance tests for the add-files application use case.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use kv_application::{
    AddFiles, AddFilesError, Clock, ClockError, FileRecord, FileStore, FileStoreError, FileSystem,
    FileSystemError, ReconcileOutcome, StoredFile,
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
    ) -> Result<ReconcileOutcome, FileStoreError> {
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

        Ok(ReconcileOutcome::new(0))
    }
}

fn collection() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

/// Covers: FR-007 and FR-013 — discovered files are added and counted.
#[test]
fn adds_discovered_files_and_reports_the_count() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("a.md", b"alpha");
    filesystem.insert("b.md", b"beta");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = AddFiles::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        &[PathBuf::from("a.md"), PathBuf::from("b.md")],
        false,
    )?;

    assert_eq!(outcome.added(), 2);
    assert_eq!(outcome.skipped(), 0);

    Ok(())
}

/// Covers: FR-011 — an unreadable path fails without ingesting.
#[test]
fn fails_without_ingesting_when_a_path_is_unreadable() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let filesystem = InMemoryFileSystem::default();
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = AddFiles::new(filesystem, store, FixedClock);

    let error = use_case
        .execute(&collection, &[PathBuf::from("missing.md")], false)
        .err()
        .ok_or_else(|| std::io::Error::other("an unreadable path should fail"))?;

    assert!(matches!(
        error,
        AddFilesError::FileSystem(FileSystemError::Unreadable { .. })
    ));

    Ok(())
}

/// Covers: FR-012 — `--force` skips unreadable paths and continues.
#[test]
fn skips_unreadable_paths_when_forced() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("notes.md", b"content");
    let mut store = InMemoryFileStore::default();
    store.add_collection(&collection);
    let mut use_case = AddFiles::new(filesystem, store, FixedClock);

    let outcome = use_case.execute(
        &collection,
        &[PathBuf::from("missing.md"), PathBuf::from("notes.md")],
        true,
    )?;

    assert_eq!(outcome.added(), 1);
    assert_eq!(outcome.skipped(), 1);

    Ok(())
}

/// Covers: FR-005 — an absent collection reports not found.
#[test]
fn reports_collection_not_found() -> Result<(), Box<dyn Error>> {
    let collection = collection()?;
    let mut filesystem = InMemoryFileSystem::default();
    filesystem.insert("a.md", b"content");
    let store = InMemoryFileStore::default();
    let mut use_case = AddFiles::new(filesystem, store, FixedClock);

    let error = use_case
        .execute(&collection, &[PathBuf::from("a.md")], false)
        .err()
        .ok_or_else(|| std::io::Error::other("an absent collection should fail"))?;

    assert!(matches!(
        error,
        AddFilesError::FileStore(FileStoreError::CollectionNotFound)
    ));

    Ok(())
}
