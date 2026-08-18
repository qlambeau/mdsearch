//! Acceptance tests for the hybrid-search application use case.

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use kv_application::{
    EmbeddingError, EmbeddingGenerator, HybridCandidate, HybridCandidates, HybridError,
    HybridSearch, HybridSearchStore, HybridSearchStoreError, Position, RerankError, Reranker,
    SearchScope,
};
use kv_domain::{
    CollectionName, Embedding, EmbeddingModel, FileId, PassageKey, PassageKind, RerankerModel,
};

fn model(name: &str) -> Result<EmbeddingModel, kv_domain::EmbeddingModelError> {
    EmbeddingModel::try_new(name)
}

fn reranker_model(name: &str) -> Result<RerankerModel, kv_domain::RerankerModelError> {
    RerankerModel::try_new(name)
}

fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(name)
}

fn key(file: u64, position: usize) -> Result<PassageKey, Box<dyn Error>> {
    Ok(PassageKey::new(
        FileId::try_new(file)?,
        PassageKind::Body,
        position,
    ))
}

fn candidate(
    file: u64,
    text: &str,
    score: f64,
    position: usize,
) -> Result<HybridCandidate, Box<dyn Error>> {
    Ok(HybridCandidate::new(
        key(file, position)?,
        collection("Notes")?,
        PathBuf::from(format!("/{file}.md")),
        PassageKind::Body,
        text.to_owned(),
        score,
        Position::new(0, 0, position + 1, position + 1),
    ))
}

#[derive(Default)]
struct FakeGenerator {
    supported: Vec<String>,
    cached: bool,
    vectors: HashMap<String, Vec<f32>>,
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
        if self.cached || download {
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
        Ok(texts
            .iter()
            .map(|text| {
                Embedding::new(
                    self.vectors
                        .get(*text)
                        .cloned()
                        .unwrap_or_else(|| vec![1.0; 3]),
                )
            })
            .collect())
    }
}

#[derive(Default)]
struct FakeStore {
    global: Option<String>,
    reranker: Option<String>,
    candidates: Option<HybridCandidates>,
    fail_not_found: bool,
    fail_not_built: bool,
    fail_stale: bool,
}

impl FakeStore {
    fn set_candidates(&mut self, candidates: HybridCandidates) {
        self.candidates = Some(candidates);
    }
}

impl HybridSearchStore for FakeStore {
    fn global_model(&self) -> Result<Option<EmbeddingModel>, HybridSearchStoreError> {
        self.global
            .as_deref()
            .map(model)
            .transpose()
            .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))
    }

    fn reranker_model(&self) -> Result<Option<kv_domain::RerankerModel>, HybridSearchStoreError> {
        self.reranker
            .as_deref()
            .map(reranker_model)
            .transpose()
            .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))
    }

    fn candidates(
        &self,
        _fts5_query: &str,
        _query_embedding: Option<&Embedding>,
        _scope: SearchScope<'_>,
        _pool: usize,
    ) -> Result<HybridCandidates, HybridSearchStoreError> {
        if self.fail_not_found {
            return Err(HybridSearchStoreError::CollectionNotFound);
        }
        if self.fail_not_built {
            return Err(HybridSearchStoreError::IndexNotBuilt);
        }
        if self.fail_stale {
            return Err(HybridSearchStoreError::StaleSemanticIndex);
        }
        self.candidates
            .clone()
            .ok_or_else(|| HybridSearchStoreError::Storage(Box::new(std::io::Error::other("none"))))
    }
}

#[derive(Default)]
struct FakeReranker {
    supported: Vec<String>,
    cached: bool,
    scores: HashMap<String, f64>,
}

impl Reranker for FakeReranker {
    fn ensure_available(&self, model: &RerankerModel, download: bool) -> Result<(), RerankError> {
        if !self.supported.iter().any(|name| name == model.as_str()) {
            return Err(RerankError::UnsupportedModel {
                model: model.as_str().to_owned(),
            });
        }
        if self.cached || download {
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
        Ok(documents
            .iter()
            .map(|document| self.scores.get(*document).copied().unwrap_or(0.0))
            .collect())
    }
}

/// Covers: FR-001 — a hybrid search returns a fused ranked list.
#[test]
fn returns_a_fused_ranked_list() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(
        vec![
            candidate(1, "lexical", 0.5, 0)?,
            candidate(2, "other", 0.2, 0)?,
        ],
        vec![candidate(1, "semantic", 0.9, 0)?],
    ));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("borrowing", 10, SearchScope::All, false)?;

    assert!(!set.reranked());
    assert!(!set.rerank_warning());
    let results = set.results();
    assert_eq!(results.len(), 2);
    let first = results
        .first()
        .ok_or_else(|| std::io::Error::other("expected a first result"))?;
    assert_eq!(first.text(), "lexical");
    assert!(first.semantic_score().is_some());
    assert!(first.lexical_score().is_some());

    Ok(())
}

/// Covers: FR-006 — an uncached re-ranker falls back to RRF with a warning.
#[test]
fn uncached_reranker_falls_back_with_a_warning() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        reranker: Some("bge-reranker-base".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(
        vec![candidate(1, "a", 0.5, 0)?],
        vec![candidate(1, "a", 0.9, 0)?],
    ));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        supported: vec!["bge-reranker-base".to_owned()],
        cached: false,
        ..FakeReranker::default()
    };
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("borrowing", 10, SearchScope::All, true)?;

    assert!(!set.reranked());
    assert!(set.rerank_warning());

    Ok(())
}

/// Covers: FR-006 — --no-rerank produces no warning.
#[test]
fn no_rerank_produces_no_warning() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(
        vec![candidate(1, "a", 0.5, 0)?],
        vec![candidate(1, "a", 0.9, 0)?],
    ));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        cached: false,
        ..FakeReranker::default()
    };
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("borrowing", 10, SearchScope::All, false)?;

    assert!(!set.reranked());
    assert!(!set.rerank_warning());

    Ok(())
}

/// Covers: FR-004 — a cached re-ranker reorders by its score.
#[test]
fn cached_reranker_reorders_by_its_score() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        reranker: Some("bge-reranker-base".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(
        vec![candidate(1, "a", 1.0, 0)?, candidate(2, "b", 0.1, 0)?],
        Vec::new(),
    ));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker {
        supported: vec!["bge-reranker-base".to_owned()],
        cached: true,
        scores: HashMap::from([("a".to_owned(), 0.1), ("b".to_owned(), 0.9)]),
    };
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("borrowing", 10, SearchScope::All, true)?;

    assert!(set.reranked());
    let results = set.results();
    let first = results
        .first()
        .ok_or_else(|| std::io::Error::other("expected a first result"))?;
    let second = results
        .get(1)
        .ok_or_else(|| std::io::Error::other("expected a second result"))?;
    assert_eq!(first.text(), "b");
    assert_eq!(second.text(), "a");

    Ok(())
}

/// Covers: FR-003 — an empty query is rejected.
#[test]
fn rejects_an_empty_query() {
    let store = FakeStore::default();
    let generator = FakeGenerator::default();
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);

    assert!(matches!(
        use_case.execute("   ", 10, SearchScope::All, false),
        Err(HybridError::EmptyQuery)
    ));
}

/// Covers: FR-010 — a stale semantic index fails.
#[test]
fn a_stale_semantic_index_fails() {
    let store = FakeStore {
        fail_stale: true,
        ..FakeStore::default()
    };
    let generator = FakeGenerator::default();
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);

    assert!(matches!(
        use_case.execute("borrowing", 10, SearchScope::All, false),
        Err(HybridError::Store(
            HybridSearchStoreError::StaleSemanticIndex
        ))
    ));
}

/// Covers: FR-009 — an unknown collection fails.
#[test]
fn an_unknown_collection_fails() -> Result<(), Box<dyn Error>> {
    let store = FakeStore {
        fail_not_found: true,
        ..FakeStore::default()
    };
    let generator = FakeGenerator::default();
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);
    let journal = collection("Journal")?;

    assert!(matches!(
        use_case.execute("borrowing", 10, SearchScope::Collection(&journal), false),
        Err(HybridError::Store(
            HybridSearchStoreError::CollectionNotFound
        ))
    ));

    Ok(())
}

/// Covers: FR-009 — an unbuilt lexical index fails.
#[test]
fn an_unbuilt_lexical_index_fails() -> Result<(), Box<dyn Error>> {
    let store = FakeStore {
        fail_not_built: true,
        ..FakeStore::default()
    };
    let generator = FakeGenerator::default();
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);
    let notes = collection("Notes")?;

    assert!(matches!(
        use_case.execute("borrowing", 10, SearchScope::Collection(&notes), false),
        Err(HybridError::Store(HybridSearchStoreError::IndexNotBuilt))
    ));

    Ok(())
}

/// Covers: FR-015 — no candidates produce an empty result set.
#[test]
fn no_candidates_produce_empty_results() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(Vec::new(), Vec::new()));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("zzznotaword", 10, SearchScope::All, false)?;

    assert!(set.results().is_empty());

    Ok(())
}

/// Covers: FR-002 — the free-text query is mapped and forwarded to the store.
#[test]
fn the_query_is_forwarded_to_the_store() -> Result<(), Box<dyn Error>> {
    let mut store = FakeStore {
        global: Some("all-MiniLM-L6-v2".to_owned()),
        ..FakeStore::default()
    };
    store.set_candidates(HybridCandidates::new(
        vec![candidate(1, "borrowing rust", 0.5, 0)?],
        vec![candidate(1, "borrowing rust", 0.9, 0)?],
    ));
    let generator = FakeGenerator {
        supported: vec!["all-MiniLM-L6-v2".to_owned()],
        cached: true,
        ..FakeGenerator::default()
    };
    let reranker = FakeReranker::default();
    let use_case = HybridSearch::new(generator, store, reranker);

    let set = use_case.execute("borrowing rust", 10, SearchScope::All, false)?;

    assert_eq!(set.results().len(), 1);
    let first = set
        .results()
        .first()
        .ok_or_else(|| std::io::Error::other("expected a result"))?;
    assert_eq!(first.text(), "borrowing rust");

    Ok(())
}
