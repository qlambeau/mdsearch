//! Acceptance tests for the index-status application use case.

use std::error::Error;

use kv_application::{
    IndexState, IndexStatus, IndexStatusError, IndexStoreError, LexicalIndexStore, ReadIndexStatus,
    SemanticStatus,
};
use kv_domain::{CollectionName, EmbeddingModel, Timestamp};

#[derive(Default)]
struct InMemoryIndexStore {
    statuses: Vec<IndexStatus>,
}

impl InMemoryIndexStore {
    fn push(&mut self, status: IndexStatus) {
        self.statuses.push(status);
    }
}

impl LexicalIndexStore for InMemoryIndexStore {
    fn status(&self) -> Result<Vec<IndexStatus>, IndexStoreError> {
        Ok(self.statuses.clone())
    }
}

fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(name)
}

/// Covers: FR-009 and FR-010 — a built collection reports counts and a timestamp.
#[test]
fn reports_a_built_index_with_counts_and_timestamp() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryIndexStore::default();
    store.push(IndexStatus::new(
        collection("Notes")?,
        3,
        12,
        Some(Timestamp::from_unix_seconds(1_700_000_000)),
        None,
    ));
    let use_case = ReadIndexStatus::new(store);

    let statuses = use_case.execute()?;

    assert_eq!(statuses.len(), 1);
    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;
    assert_eq!(status.collection().display_name(), "Notes");
    assert_eq!(status.state(), IndexState::Built);
    assert_eq!(status.file_count(), 3);
    assert_eq!(status.passage_count(), 12);
    assert_eq!(
        status.built_at(),
        Some(Timestamp::from_unix_seconds(1_700_000_000))
    );

    Ok(())
}

/// Covers: FR-010 — a never-built collection reports `NotBuilt` with no timestamp.
#[test]
fn reports_a_not_built_index() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryIndexStore::default();
    store.push(IndexStatus::new(collection("Notes")?, 2, 0, None, None));
    let use_case = ReadIndexStatus::new(store);

    let statuses = use_case.execute()?;

    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;
    assert_eq!(status.state(), IndexState::NotBuilt);
    assert_eq!(status.passage_count(), 0);
    assert_eq!(status.built_at(), None);

    Ok(())
}

/// Covers: FR-015 — a database with no collections reports nothing.
#[test]
fn reports_nothing_for_no_collections() -> Result<(), Box<dyn Error>> {
    let store = InMemoryIndexStore::default();
    let use_case = ReadIndexStatus::new(store);

    let statuses = use_case.execute()?;

    assert!(statuses.is_empty());

    Ok(())
}

/// Covers: the index-status error path.
#[test]
fn propagates_an_index_store_error() {
    struct FailingStore;

    impl LexicalIndexStore for FailingStore {
        fn status(&self) -> Result<Vec<IndexStatus>, IndexStoreError> {
            Err(IndexStoreError::Storage(Box::new(std::io::Error::other(
                "forced failure",
            ))))
        }
    }

    let use_case = ReadIndexStatus::new(FailingStore);

    assert!(matches!(
        use_case.execute(),
        Err(IndexStatusError::Store(IndexStoreError::Storage(_)))
    ));
}

/// Covers: REQ-006 FR-017 — a status carries the semantic model and dimension
/// when the collection has a recorded semantic state.
#[test]
fn reports_the_semantic_model_and_dimension_when_embedded() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryIndexStore::default();
    store.push(IndexStatus::new(
        collection("Notes")?,
        3,
        12,
        Some(Timestamp::from_unix_seconds(1_700_000_000)),
        Some(SemanticStatus::new(
            EmbeddingModel::try_new("bge-large-en-v1.5")?,
            1024,
        )),
    ));
    let use_case = ReadIndexStatus::new(store);

    let statuses = use_case.execute()?;
    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;
    let semantic = status
        .semantic()
        .ok_or_else(|| std::io::Error::other("expected semantic status"))?;

    assert_eq!(semantic.model().as_str(), "bge-large-en-v1.5");
    assert_eq!(semantic.dimension(), 1024);

    Ok(())
}

/// Covers: REQ-006 FR-017 — a collection without semantic state reports none.
#[test]
fn reports_no_semantic_status_without_a_semantic_state() -> Result<(), Box<dyn Error>> {
    let mut store = InMemoryIndexStore::default();
    store.push(IndexStatus::new(collection("Notes")?, 2, 0, None, None));
    let use_case = ReadIndexStatus::new(store);

    let statuses = use_case.execute()?;
    let status = statuses
        .first()
        .ok_or_else(|| std::io::Error::other("expected one status"))?;

    assert_eq!(status.semantic(), None);

    Ok(())
}
