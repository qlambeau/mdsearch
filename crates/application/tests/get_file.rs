//! Acceptance tests for the get-file application use case.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use kv_application::{
    FileRetrievalStore, FileRetrievalStoreError, GetFile, GetFileError, RetrievedFile,
};
use kv_domain::{CollectionName, FileId};

#[derive(Default)]
struct InMemoryRetrievalStore {
    files: BTreeMap<String, Vec<RetrievedFile>>,
}

impl InMemoryRetrievalStore {
    fn store(&mut self, collection: &str, path: &str, content: &str) {
        self.files
            .entry(collection.to_owned())
            .or_default()
            .push(RetrievedFile::new(
                PathBuf::from(path),
                content.as_bytes().to_vec(),
            ));
    }
}

impl FileRetrievalStore for InMemoryRetrievalStore {
    fn get_by_path(
        &self,
        collection: &CollectionName,
        path: &std::path::Path,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError> {
        Ok(self
            .files
            .get(collection.name_key())
            .and_then(|files| files.iter().find(|file| file.path() == path).cloned()))
    }

    fn get_by_id(
        &self,
        collection: &CollectionName,
        id: FileId,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError> {
        let index = usize::try_from(id.as_u64())
            .ok()
            .and_then(|index| index.checked_sub(1));
        Ok(index.and_then(|index| {
            self.files
                .get(collection.name_key())
                .and_then(|files| files.get(index).cloned())
        }))
    }

    fn list_by_basename(
        &self,
        collection: &CollectionName,
        basename: &str,
    ) -> Result<Vec<RetrievedFile>, FileRetrievalStoreError> {
        Ok(self
            .files
            .get(collection.name_key())
            .map(|files| {
                files
                    .iter()
                    .filter(|file| {
                        file.path().file_name().and_then(|name| name.to_str()) == Some(basename)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }
}

fn collection() -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from("Notes")
}

/// Covers: FR-004 — retrieval by exact canonical path.
#[test]
fn retrieves_by_exact_path() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/vault/notes.md", "alpha");
    let use_case = GetFile::new(store);

    let file = use_case.execute(&collection()?, "/vault/notes.md")?;

    assert_eq!(file.content(), b"alpha");

    Ok(())
}

/// Covers: FR-004 — retrieval by a unique basename.
#[test]
fn retrieves_by_unique_basename() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/vault/notes.md", "alpha");
    let use_case = GetFile::new(store);

    let file = use_case.execute(&collection()?, "notes.md")?;

    assert_eq!(file.content(), b"alpha");

    Ok(())
}

/// Covers: FR-003 — retrieval by an indexing-assigned ID.
#[test]
fn retrieves_by_id() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/vault/notes.md", "alpha");
    store.store("notes", "/vault/other.md", "beta");
    let use_case = GetFile::new(store);

    let file = use_case.execute(&collection()?, "1")?;

    assert_eq!(file.content(), b"alpha");

    Ok(())
}

/// Covers: FR-005 — an ambiguous basename lists the candidate paths.
#[test]
fn reports_an_ambiguous_basename_with_candidates() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/a/x.md", "one");
    store.store("notes", "/b/x.md", "two");
    let use_case = GetFile::new(store);

    let error = use_case
        .execute(&collection()?, "x.md")
        .err()
        .ok_or_else(|| std::io::Error::other("an ambiguous basename should fail"))?;

    match error {
        GetFileError::Ambiguous(paths) => {
            assert_eq!(paths.len(), 2);
            assert!(paths.iter().any(|path| path.ends_with("/a/x.md")));
            assert!(paths.iter().any(|path| path.ends_with("/b/x.md")));
        }
        other => return Err(std::io::Error::other(format!("unexpected error: {other:?}")).into()),
    }

    Ok(())
}

/// Covers: FR-006 — a name with no match reports not found.
#[test]
fn reports_not_found_by_name() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/vault/notes.md", "alpha");
    let use_case = GetFile::new(store);

    let error = use_case
        .execute(&collection()?, "missing.md")
        .err()
        .ok_or_else(|| std::io::Error::other("a missing file should fail"))?;

    assert!(matches!(error, GetFileError::FileNotFound));

    Ok(())
}

/// Covers: FR-006 — an ID with no match reports not found.
#[test]
fn reports_not_found_by_id() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryRetrievalStore::default();
    store.store("notes", "/vault/notes.md", "alpha");
    let use_case = GetFile::new(store);

    let error = use_case
        .execute(&collection()?, "999")
        .err()
        .ok_or_else(|| std::io::Error::other("a missing file should fail"))?;

    assert!(matches!(error, GetFileError::FileNotFound));

    Ok(())
}
