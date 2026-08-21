//! Acceptance tests for the embed-collections application use case.

use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use kv_application::{
    ClockError, EmbedCollections, EmbedOutcome, EmbedReport, EmbedScope, EmbeddingError,
    EmbeddingGenerator, RerankError, Reranker, SemanticIndexStore, SemanticIndexStoreError,
    SkipReason,
};
use kv_domain::{
    CollectionName, ContentHash, Embedding, EmbeddingModel, FileId, RerankerModel,
    SemanticIndexStatus, SemanticPassage, Timestamp,
};

#[derive(Default)]
struct FakeClock {
    now: u64,
}

impl kv_application::Clock for FakeClock {
    fn now(&self) -> Result<Timestamp, ClockError> {
        Ok(Timestamp::from_unix_seconds(self.now))
    }
}

#[derive(Default)]
struct FakeGenerator {
    available: bool,
    download_allowed: bool,
    supported: Vec<String>,
    vectors: HashMap<String, Vec<f32>>,
    fail_embed: bool,
}

fn model(name: &str) -> Result<EmbeddingModel, kv_domain::EmbeddingModelError> {
    EmbeddingModel::try_new(name)
}

impl EmbeddingGenerator for FakeGenerator {
    fn ensure_available(
        &self,
        model: &EmbeddingModel,
        download: bool,
    ) -> Result<(), EmbeddingError> {
        if !self.supported.iter().any(|name| name == model.as_str()) {
            return Err(EmbeddingError::UnsupportedModel {
                model: model.as_str().to_owned(),
            });
        }
        if self.available || (download && self.download_allowed) {
            Ok(())
        } else {
            Err(EmbeddingError::ModelNotCached {
                model: model.as_str().to_owned(),
            })
        }
    }

    fn embed(
        &self,
        _model: &EmbeddingModel,
        texts: &[&str],
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        if self.fail_embed {
            return Err(EmbeddingError::Storage(Box::new(std::io::Error::other(
                "embedding failed",
            ))));
        }
        Ok(texts
            .iter()
            .map(|text| {
                let values = self
                    .vectors
                    .get(*text)
                    .cloned()
                    .unwrap_or_else(|| vec![1.0; 2]);
                Embedding::new(values)
            })
            .collect())
    }
}

#[derive(Default)]
struct FakeReranker {
    supported: Vec<String>,
    cached: bool,
    download_allowed: bool,
}

impl Reranker for FakeReranker {
    fn ensure_available(&self, model: &RerankerModel, download: bool) -> Result<(), RerankError> {
        if !self.supported.iter().any(|name| name == model.as_str()) {
            return Err(RerankError::UnsupportedModel {
                model: model.as_str().to_owned(),
            });
        }
        if self.cached || (download && self.download_allowed) {
            Ok(())
        } else {
            Err(RerankError::ModelNotCached {
                model: model.as_str().to_owned(),
            })
        }
    }

    fn rerank(
        &self,
        _model: &RerankerModel,
        _query: &str,
        documents: &[&str],
    ) -> Result<Vec<f64>, RerankError> {
        Ok(documents.iter().map(|_| 0.0).collect())
    }
}

struct FakeStore {
    targets: Vec<(String, bool, bool)>,
    global: Option<String>,
    reranker: Option<String>,
    statuses: HashMap<String, Option<(String, String, usize, u64)>>,
    fingerprints: HashMap<String, String>,
    passages: HashMap<String, Vec<(u64, String, usize, String)>>,
    embedded: Vec<String>,
    rebuilds: RefCell<Vec<(String, String, usize)>>,
    fail_store: bool,
    fail_rebuild: bool,
    recorded_model: RefCell<Option<String>>,
    recorded_reranker: Rc<RefCell<Option<String>>>,
    recorded_dimension: RefCell<Option<usize>>,
}

impl Default for FakeStore {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            global: None,
            reranker: None,
            statuses: HashMap::new(),
            fingerprints: HashMap::new(),
            passages: HashMap::new(),
            embedded: Vec::new(),
            rebuilds: RefCell::new(Vec::new()),
            fail_store: false,
            fail_rebuild: false,
            recorded_model: RefCell::new(None),
            recorded_reranker: Rc::new(RefCell::new(None)),
            recorded_dimension: RefCell::new(None),
        }
    }
}

impl FakeStore {
    fn model(name: &str) -> Result<EmbeddingModel, kv_domain::EmbeddingModelError> {
        model(name)
    }
}

impl SemanticIndexStore for FakeStore {
    fn targets(&self) -> Result<Vec<kv_application::EmbedTarget>, SemanticIndexStoreError> {
        self.targets
            .iter()
            .map(|(name, has_files, lexical_built)| {
                let collection = CollectionName::try_from(name.as_str())
                    .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))?;
                Ok(kv_application::EmbedTarget::new(
                    collection,
                    *has_files,
                    *lexical_built,
                ))
            })
            .collect()
    }

    fn resolve(
        &self,
        collection: &CollectionName,
    ) -> Result<kv_application::EmbedTarget, SemanticIndexStoreError> {
        self.targets
            .iter()
            .find(|(name, _, _)| name.eq_ignore_ascii_case(collection.display_name()))
            .map(|(name, has_files, lexical_built)| {
                Ok(kv_application::EmbedTarget::new(
                    CollectionName::try_from(name.as_str())
                        .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))?,
                    *has_files,
                    *lexical_built,
                ))
            })
            .transpose()?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)
    }

    fn global_model(&self) -> Result<Option<EmbeddingModel>, SemanticIndexStoreError> {
        self.global
            .as_deref()
            .map(Self::model)
            .transpose()
            .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))
    }

    fn set_global_model(&mut self, model: &EmbeddingModel) -> Result<(), SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        *self.recorded_model.borrow_mut() = Some(model.as_str().to_owned());
        Ok(())
    }

    fn reranker_model(&self) -> Result<Option<RerankerModel>, SemanticIndexStoreError> {
        self.reranker
            .as_deref()
            .map(RerankerModel::try_new)
            .transpose()
            .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))
    }

    fn set_reranker_model(&mut self, model: &RerankerModel) -> Result<(), SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        *self.recorded_reranker.borrow_mut() = Some(model.as_str().to_owned());
        Ok(())
    }

    fn ensure_dimension(&mut self, dimension: usize) -> Result<(), SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        *self.recorded_dimension.borrow_mut() = Some(dimension);
        Ok(())
    }

    fn status(
        &self,
        collection: &CollectionName,
    ) -> Result<Option<SemanticIndexStatus>, SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        self.statuses
            .get(collection.display_name())
            .cloned()
            .flatten()
            .map(|(fingerprint, model, count, at)| {
                Ok(SemanticIndexStatus::new(
                    ContentHash::from_content(fingerprint.as_bytes()),
                    Self::model(&model)
                        .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))?,
                    384,
                    count,
                    Timestamp::from_unix_seconds(at),
                ))
            })
            .transpose()
    }

    fn embedded_collections(&self) -> Result<Vec<CollectionName>, SemanticIndexStoreError> {
        self.embedded
            .iter()
            .map(|name| {
                CollectionName::try_from(name.as_str())
                    .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))
            })
            .collect()
    }

    fn file_set_fingerprint(
        &self,
        collection: &CollectionName,
    ) -> Result<ContentHash, SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        self.fingerprints
            .get(collection.display_name())
            .map(|value| ContentHash::from_content(value.as_bytes()))
            .ok_or(SemanticIndexStoreError::CollectionNotFound)
    }

    fn passages(
        &self,
        collection: &CollectionName,
    ) -> Result<Vec<SemanticPassage>, SemanticIndexStoreError> {
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        self.passages
            .get(collection.display_name())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|(file, kind, position, text)| {
                let file = FileId::try_new(file)
                    .map_err(|error| SemanticIndexStoreError::Storage(Box::new(error)))?;
                let kind = kv_domain::PassageKind::from_key(&kind).ok_or_else(|| {
                    SemanticIndexStoreError::Storage(Box::new(std::io::Error::other(
                        "unknown kind",
                    )))
                })?;
                Ok(SemanticPassage::new(file, kind, position, text))
            })
            .collect()
    }

    fn rebuild(
        &mut self,
        collection: &CollectionName,
        model: &EmbeddingModel,
        _embedded_at: Timestamp,
        embeddings: &[(SemanticPassage, Embedding)],
    ) -> Result<usize, SemanticIndexStoreError> {
        if self.fail_rebuild {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("rebuild failed"),
            )));
        }
        if self.fail_store {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other("store failed"),
            )));
        }
        let count = embeddings.len();
        self.rebuilds.borrow_mut().push((
            collection.display_name().to_owned(),
            model.as_str().to_owned(),
            count,
        ));
        Ok(count)
    }
}

fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(name)
}

fn supported(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}

fn fingerprint(value: &str) -> String {
    value.to_owned()
}

/// Covers: FR-001 and FR-016 — embedding all collections reports passage counts.
#[test]
fn embeds_every_eligible_collection() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert_eq!(report.outcomes().len(), 1);
    assert!(!report.any_failed());
    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Embedded {
            collection,
            passage_count: 1,
        }) if collection.display_name() == "Notes"
    ));

    Ok(())
}

/// Covers: FR-004 — an unchanged collection is reported already current.
#[test]
fn reports_an_unchanged_collection_as_already_current() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store.statuses.insert(
        "Notes".to_owned(),
        Some(("files".to_owned(), "all-MiniLM-L6-v2".to_owned(), 1, 1)),
    );
    store.global = Some("all-MiniLM-L6-v2".to_owned());
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 2 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::AlreadyCurrent { collection })
            if collection.display_name() == "Notes"
    ));

    Ok(())
}

/// Covers: FR-004 — a changed file set triggers a rebuild.
#[test]
fn rebuilds_when_the_file_set_changed() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("new-files"));
    store.statuses.insert(
        "Notes".to_owned(),
        Some(("old-files".to_owned(), "all-MiniLM-L6-v2".to_owned(), 1, 1)),
    );
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store.global = Some("all-MiniLM-L6-v2".to_owned());
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 2 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Embedded {
            passage_count: 1,
            ..
        })
    ));

    Ok(())
}

/// Covers: FR-006 and FR-007 — a --model switch rebuilds every embedded collection.
#[test]
fn model_switch_rebuilds_every_embedded_collection() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.embedded.push("Archive".to_owned());
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store.statuses.insert(
        "Notes".to_owned(),
        Some(("files".to_owned(), "alpha".to_owned(), 1, 1)),
    );
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store.global = Some("alpha".to_owned());
    let generator = FakeGenerator {
        available: true,
        supported: vec!["alpha".to_owned(), "beta".to_owned()],
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 2 },
        FakeReranker::default(),
    );
    let beta = model("beta")?;

    let report = use_case.execute(EmbedScope::All, Some(&beta), None, false, &mut |_| {})?;

    let names = report
        .outcomes()
        .iter()
        .map(EmbedOutcome::collection)
        .map(CollectionName::display_name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"Notes"));
    assert!(names.contains(&"Archive"));

    Ok(())
}

/// Covers: FR-007 — a model switch rebuilds embedded collections under a narrow scope.
#[test]
fn model_switch_rebuilds_embedded_collections_even_under_a_narrow_scope()
-> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.embedded.push("Archive".to_owned());
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store.statuses.insert(
        "Notes".to_owned(),
        Some(("files".to_owned(), "alpha".to_owned(), 1, 1)),
    );
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store.global = Some("alpha".to_owned());
    let generator = FakeGenerator {
        available: true,
        supported: vec!["alpha".to_owned(), "beta".to_owned()],
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 2 },
        FakeReranker::default(),
    );
    let beta = model("beta")?;
    let notes = collection("Notes")?;

    let report = use_case.execute(
        EmbedScope::Collection(&notes),
        Some(&beta),
        None,
        false,
        &mut |_| {},
    )?;

    let names = report
        .outcomes()
        .iter()
        .map(EmbedOutcome::collection)
        .map(CollectionName::display_name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"Notes"));
    assert!(names.contains(&"Archive"));

    Ok(())
}

/// Covers: FR-008 — an unsupported model fails before any collection work.
#[test]
fn unsupported_model_fails_before_any_collection_work() -> Result<(), Box<dyn Error>> {
    let store = FakeStore::default();
    let generator = FakeGenerator {
        supported: Vec::new(),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock::default(),
        FakeReranker::default(),
    );
    let bogus = model("bogus")?;

    assert!(
        use_case
            .execute(EmbedScope::All, Some(&bogus), None, false, &mut |_| {})
            .is_err()
    );

    Ok(())
}

/// Covers: FR-009 — a missing local model fails before any collection work.
#[test]
fn missing_local_model_fails_before_any_collection_work() {
    let store = FakeStore::default();
    let generator = FakeGenerator {
        available: false,
        download_allowed: false,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock::default(),
        FakeReranker::default(),
    );

    assert!(
        use_case
            .execute(EmbedScope::All, None, None, false, &mut |_| {})
            .is_err()
    );
}

/// Covers: FR-010 — --download allows an uncached model to proceed.
#[test]
fn download_allows_an_uncached_model() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    let generator = FakeGenerator {
        available: false,
        download_allowed: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, true, &mut |_| {})?;

    assert!(!report.any_failed());
    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Embedded { .. })
    ));

    Ok(())
}

/// Covers: FR-011 — an unbuilt lexical index is skipped in all-collections mode.
#[test]
fn skips_a_collection_without_a_built_lexical_index() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, false));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Skipped {
            reason: SkipReason::LexicalNotBuilt,
            ..
        })
    ));

    Ok(())
}

/// Covers: FR-012 — an unbuilt lexical index fails when explicitly targeted.
#[test]
fn unbuilt_lexical_index_fails_when_explicitly_targeted() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Archive".to_owned(), true, false));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );
    let archive = collection("Archive")?;

    assert!(matches!(
        use_case.execute(
            EmbedScope::Collection(&archive),
            None,
            None,
            false,
            &mut |_| {}
        ),
        Err(kv_application::EmbedError::IndexNotBuilt)
    ));

    Ok(())
}

/// Covers: FR-013 — a collection with no files is skipped and reported.
#[test]
fn skips_a_collection_with_no_files() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Empty".to_owned(), false, true));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Skipped {
            reason: SkipReason::NoFiles,
            ..
        })
    ));

    Ok(())
}

/// Covers: FR-013 — a no-files collection is skipped even when targeted.
#[test]
fn skips_a_no_files_collection_even_when_targeted() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Empty".to_owned(), false, true));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );
    let empty = collection("Empty")?;

    let report = use_case.execute(
        EmbedScope::Collection(&empty),
        None,
        None,
        false,
        &mut |_| {},
    )?;

    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Skipped {
            reason: SkipReason::NoFiles,
            ..
        })
    ));

    Ok(())
}

/// Covers: FR-014 — an unknown collection fails when explicitly targeted.
#[test]
fn unknown_collection_fails_when_explicitly_targeted() -> Result<(), Box<dyn Error>> {
    let store = FakeStore::default();
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );
    let journal = collection("Journal")?;

    assert!(matches!(
        use_case.execute(
            EmbedScope::Collection(&journal),
            None,
            None,
            false,
            &mut |_| {}
        ),
        Err(kv_application::EmbedError::CollectionNotFound)
    ));

    Ok(())
}

/// Covers: FR-015 — a per-collection failure is reported and processing continues.
#[test]
fn per_collection_failure_is_reported_and_processing_continues() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.targets.push(("Archive".to_owned(), true, true));
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store
        .fingerprints
        .insert("Archive".to_owned(), fingerprint("files"));
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store.passages.insert(
        "Archive".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        fail_embed: true,
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert_eq!(report.outcomes().len(), 2);
    assert!(report.any_failed());
    assert_eq!(
        report
            .outcomes()
            .iter()
            .filter(|outcome| outcome.is_failed())
            .count(),
        2
    );

    Ok(())
}

/// Covers: FR-005 — a failed rebuild reports a failure for that collection.
#[test]
fn failed_rebuild_reports_a_failure() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    store.passages.insert(
        "Notes".to_owned(),
        vec![(1, "body".to_owned(), 0, "borrowing".to_owned())],
    );
    store.fail_rebuild = true;
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(report.any_failed());
    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Failed { collection, .. })
            if collection.display_name() == "Notes"
    ));

    Ok(())
}

/// Guards: the report carries no outcome when no collections are eligible.
#[test]
fn empty_report_when_no_collections_are_eligible() -> Result<(), Box<dyn Error>> {
    let store = FakeStore::default();
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut |_| {})?;

    assert!(report.outcomes().is_empty());

    Ok(())
}

/// Covers: REQ-011 FR-017 — a reranker is provisioned and recorded.
#[test]
fn provisions_and_records_a_reranker() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    let recorded = store.recorded_reranker.clone();
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        supported: supported("bge-reranker-base"),
        cached: true,
        ..FakeReranker::default()
    };
    let mut use_case = EmbedCollections::new(generator, store, FakeClock { now: 1 }, reranker);
    let reranker_name = RerankerModel::try_new("bge-reranker-base")?;

    use_case.execute(
        EmbedScope::All,
        None,
        Some(&reranker_name),
        false,
        &mut |_| {},
    )?;

    assert_eq!(*recorded.borrow(), Some("bge-reranker-base".to_owned()));

    Ok(())
}

/// Covers: REQ-011 FR-021 — an unsupported reranker fails before collection work.
#[test]
fn an_unsupported_reranker_fails() -> Result<(), Box<dyn Error>> {
    let store = FakeStore::default();
    let generator = FakeGenerator {
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        supported: Vec::new(),
        ..FakeReranker::default()
    };
    let mut use_case = EmbedCollections::new(generator, store, FakeClock::default(), reranker);
    let bogus = RerankerModel::try_new("bogus")?;

    assert!(matches!(
        use_case.execute(EmbedScope::All, None, Some(&bogus), false, &mut |_| {}),
        Err(kv_application::EmbedError::Reranker(
            RerankError::UnsupportedModel { .. }
        ))
    ));

    Ok(())
}

/// Covers: REQ-011 FR-021 — an uncached reranker fails without download.
#[test]
fn an_uncached_reranker_fails_without_download() -> Result<(), Box<dyn Error>> {
    let store = FakeStore::default();
    let generator = FakeGenerator::default();
    let reranker = FakeReranker {
        supported: supported("bge-reranker-base"),
        cached: false,
        download_allowed: false,
    };
    let mut use_case = EmbedCollections::new(generator, store, FakeClock::default(), reranker);
    let name = RerankerModel::try_new("bge-reranker-base")?;

    assert!(matches!(
        use_case.execute(EmbedScope::All, None, Some(&name), false, &mut |_| {}),
        Err(kv_application::EmbedError::Reranker(
            RerankError::ModelNotCached { .. }
        ))
    ));

    Ok(())
}

/// Covers: REQ-011 FR-020 — --download allows an uncached reranker to proceed.
#[test]
fn download_allows_an_uncached_reranker() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        supported: supported("bge-reranker-base"),
        cached: false,
        download_allowed: true,
    };
    let mut use_case = EmbedCollections::new(generator, store, FakeClock { now: 1 }, reranker);
    let name = RerankerModel::try_new("bge-reranker-base")?;

    let report = use_case.execute(EmbedScope::All, None, Some(&name), true, &mut |_| {})?;

    assert!(!report.any_failed());

    Ok(())
}

/// Guards: an empty report has no failures.
#[test]
fn empty_report_has_no_failures() {
    let report = EmbedReport::new();

    assert!(!report.any_failed());
    assert!(report.outcomes().is_empty());
}

/// Covers: REQ-018 FR-001/FR-002/FR-005 — per-file progress events and a
/// Writing event are emitted during embedding.
#[test]
fn reports_per_file_progress_during_embedding() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.passages.insert(
        "Notes".to_owned(),
        vec![
            (1, "body".to_owned(), 0, "borrowing".to_owned()),
            (1, "body".to_owned(), 1, "ownership".to_owned()),
            (2, "body".to_owned(), 0, "lifetimes".to_owned()),
            (3, "body".to_owned(), 0, "traits".to_owned()),
        ],
    );
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let events: Rc<RefCell<Vec<String>>> = Rc::default();
    let recorded = events.clone();
    let mut progress = move |event: kv_application::EmbedProgress| match event {
        kv_application::EmbedProgress::Files {
            collection,
            completed_files,
            total_files,
        } => {
            recorded.borrow_mut().push(format!(
                "{}:{}/{}",
                collection.display_name(),
                completed_files,
                total_files
            ));
        }
        kv_application::EmbedProgress::Writing { collection } => {
            recorded
                .borrow_mut()
                .push(format!("writing:{}", collection.display_name()));
        }
    };

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut progress)?;

    assert_eq!(
        *events.borrow(),
        vec![
            "Notes:1/3".to_owned(),
            "Notes:2/3".to_owned(),
            "Notes:3/3".to_owned(),
            "writing:Notes".to_owned(),
        ]
    );
    assert_eq!(report.outcomes().len(), 1);

    Ok(())
}

/// Covers: REQ-018 FR-003 — skipped and already-current collections emit no
/// progress events.
#[test]
fn emits_no_progress_for_skipped_or_current_collections() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Empty".to_owned(), false, true));
    store.targets.push(("Current".to_owned(), true, true));
    store
        .fingerprints
        .insert("Current".to_owned(), fingerprint("files"));
    store.statuses.insert(
        "Current".to_owned(),
        Some(("files".to_owned(), "all-MiniLM-L6-v2".to_owned(), 1, 1)),
    );
    store.global = Some("all-MiniLM-L6-v2".to_owned());
    let generator = FakeGenerator {
        available: true,
        supported: supported("all-MiniLM-L6-v2"),
        ..FakeGenerator::default()
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let events: Rc<RefCell<Vec<String>>> = Rc::default();
    let recorded = events.clone();
    let mut progress = move |event: kv_application::EmbedProgress| match event {
        kv_application::EmbedProgress::Files {
            collection,
            completed_files,
            total_files,
        } => {
            recorded.borrow_mut().push(format!(
                "{}:{}/{}",
                collection.display_name(),
                completed_files,
                total_files
            ));
        }
        kv_application::EmbedProgress::Writing { collection } => {
            recorded
                .borrow_mut()
                .push(format!("writing:{}", collection.display_name()));
        }
    };

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut progress)?;

    assert!(events.borrow().is_empty());
    assert_eq!(report.outcomes().len(), 2);

    Ok(())
}

/// Covers: REQ-018 FR-007 — a mid-run embedding failure stops progress events
/// and yields the Failed outcome.
#[test]
fn stops_progress_events_when_embedding_fails() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore::default();
    store.targets.push(("Notes".to_owned(), true, true));
    store.passages.insert(
        "Notes".to_owned(),
        vec![
            (1, "body".to_owned(), 0, "alpha".to_owned()),
            (2, "body".to_owned(), 0, "boom text".to_owned()),
            (3, "body".to_owned(), 0, "gamma".to_owned()),
        ],
    );
    store
        .fingerprints
        .insert("Notes".to_owned(), fingerprint("files"));
    let generator = FailOnTextGenerator {
        inner: FakeGenerator {
            available: true,
            supported: supported("all-MiniLM-L6-v2"),
            ..FakeGenerator::default()
        },
        fail_on_text: "boom text".to_owned(),
    };
    let mut use_case = EmbedCollections::new(
        generator,
        store,
        FakeClock { now: 1 },
        FakeReranker::default(),
    );

    let events: Rc<RefCell<Vec<String>>> = Rc::default();
    let recorded = events.clone();
    let mut progress = move |event: kv_application::EmbedProgress| match event {
        kv_application::EmbedProgress::Files {
            collection,
            completed_files,
            total_files,
        } => {
            recorded.borrow_mut().push(format!(
                "{}:{}/{}",
                collection.display_name(),
                completed_files,
                total_files
            ));
        }
        kv_application::EmbedProgress::Writing { collection } => {
            recorded
                .borrow_mut()
                .push(format!("writing:{}", collection.display_name()));
        }
    };

    let report = use_case.execute(EmbedScope::All, None, None, false, &mut progress)?;

    assert_eq!(*events.borrow(), vec!["Notes:1/3".to_owned()]);
    assert!(matches!(
        report.outcomes().first(),
        Some(EmbedOutcome::Failed { collection, .. })
            if collection.display_name() == "Notes"
    ));

    Ok(())
}

/// A generator that fails when the batch contains a designated text.
struct FailOnTextGenerator {
    inner: FakeGenerator,
    fail_on_text: String,
}

impl EmbeddingGenerator for FailOnTextGenerator {
    fn ensure_available(
        &self,
        model: &EmbeddingModel,
        download: bool,
    ) -> Result<(), EmbeddingError> {
        self.inner.ensure_available(model, download)
    }

    fn embed(
        &self,
        model: &EmbeddingModel,
        texts: &[&str],
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.iter().any(|text| text.contains(&self.fail_on_text)) {
            return Err(EmbeddingError::Storage(Box::new(std::io::Error::other(
                "embedding failed",
            ))));
        }
        self.inner.embed(model, texts)
    }
}
