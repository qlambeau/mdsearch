//! Contract tests for the re-ranker port.

use std::collections::HashMap;
use std::error::Error;

use kv_application::RerankError;
use kv_application::Reranker;
use kv_domain::RerankerModel;

/// Deterministic in-memory re-ranker test double.
#[derive(Default)]
struct InMemoryReranker {
    supported: Vec<String>,
    cached: bool,
    download_allowed: bool,
    scores: HashMap<String, f64>,
    fail_rerank: bool,
}

impl Reranker for InMemoryReranker {
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
        if self.fail_rerank {
            return Err(RerankError::Storage(Box::new(std::io::Error::other(
                "re-ranking failed",
            ))));
        }
        Ok(documents
            .iter()
            .map(|document| self.scores.get(*document).copied().unwrap_or(0.0))
            .collect())
    }
}

fn model(name: &str) -> Result<RerankerModel, kv_domain::RerankerModelError> {
    RerankerModel::try_new(name)
}

fn supported(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}

/// Covers: REQ-011 FR-006 — an uncached model fails without download.
#[test]
fn uncached_model_fails_without_download() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker {
        supported: supported("bge-reranker-base"),
        cached: false,
        ..InMemoryReranker::default()
    };
    let name = model("bge-reranker-base")?;

    assert!(matches!(
        reranker.ensure_available(&name, false),
        Err(RerankError::ModelNotCached { .. })
    ));

    Ok(())
}

/// Covers: REQ-011 FR-006 — a download-allowed fake satisfies an uncached model.
#[test]
fn download_satisfies_an_uncached_model() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker {
        supported: supported("bge-reranker-base"),
        cached: false,
        download_allowed: true,
        ..InMemoryReranker::default()
    };
    let name = model("bge-reranker-base")?;

    assert!(reranker.ensure_available(&name, true).is_ok());

    Ok(())
}

/// Covers: REQ-011 — an unsupported model is rejected.
#[test]
fn unsupported_model_is_rejected() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker {
        supported: Vec::new(),
        ..InMemoryReranker::default()
    };
    let bogus = model("bogus")?;

    assert!(matches!(
        reranker.ensure_available(&bogus, false),
        Err(RerankError::UnsupportedModel { .. })
    ));

    Ok(())
}

/// Covers: REQ-011 FR-004 — re-ranking returns one score per document.
#[test]
fn rerank_returns_one_score_per_document() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker {
        scores: HashMap::from([
            ("borrowing rules".to_owned(), 0.9),
            ("borrowing quirks".to_owned(), 0.4),
        ]),
        ..InMemoryReranker::default()
    };
    let name = model("bge-reranker-base")?;
    let documents = ["borrowing rules", "borrowing quirks"];

    let scores = reranker.rerank(&name, "borrowing", &documents)?;

    assert_eq!(scores, vec![0.9, 0.4]);

    Ok(())
}

/// Covers: REQ-011 — missing documents score zero.
#[test]
fn missing_documents_score_zero() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker::default();
    let name = model("bge-reranker-base")?;

    let scores = reranker.rerank(&name, "borrowing", &["unknown"])?;

    assert_eq!(scores, vec![0.0]);

    Ok(())
}

/// Covers: REQ-011 — a storage failure propagates.
#[test]
fn storage_failure_propagates() -> Result<(), Box<dyn Error>> {
    let reranker = InMemoryReranker {
        fail_rerank: true,
        ..InMemoryReranker::default()
    };
    let name = model("bge-reranker-base")?;

    assert!(matches!(
        reranker.rerank(&name, "borrowing", &["rules"]),
        Err(RerankError::Storage(_))
    ));

    Ok(())
}
