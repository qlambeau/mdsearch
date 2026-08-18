use std::cell::RefCell;
use std::path::{Path, PathBuf};

use fastembed::{RerankInitOptions, RerankerModel as FastembedRerankerModel, TextRerank};
use kv_application::Reranker;
use kv_domain::RerankerModel;

/// Re-scores candidate documents locally with `fastembed`'s `TextRerank`.
pub struct FastembedReranker {
    session: RefCell<Option<TextRerank>>,
    cache_dir: Option<PathBuf>,
}

impl FastembedReranker {
    /// Creates a re-ranker that loads models from fastembed's cache.
    ///
    /// When `cache_dir` is `None`, the cache directory is resolved from the
    /// `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then the default
    /// `.fastembed_cache` directory, matching fastembed's resolution order.
    #[must_use]
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self {
            session: RefCell::new(None),
            cache_dir,
        }
    }
}

impl Reranker for FastembedReranker {
    fn ensure_available(
        &self,
        model: &RerankerModel,
        download: bool,
    ) -> Result<(), kv_application::RerankError> {
        let fastembed_model = resolve_reranker_model(model)?;
        let cache_dir = self.effective_cache_dir();

        if model_is_cached(&cache_dir, &fastembed_model) {
            return Ok(());
        }

        if !download {
            return Err(kv_application::RerankError::ModelNotCached {
                model: model.as_str().to_owned(),
            });
        }

        let session = build_rerank_session(fastembed_model, &cache_dir)?;
        *self.session.borrow_mut() = Some(session);

        Ok(())
    }

    fn rerank(
        &self,
        model: &RerankerModel,
        query: &str,
        documents: &[&str],
    ) -> Result<Vec<f64>, kv_application::RerankError> {
        let fastembed_model = resolve_reranker_model(model)?;

        let mut session = self.session.borrow_mut();
        if session.is_none() {
            let cache_dir = self.effective_cache_dir();
            if !model_is_cached(&cache_dir, &fastembed_model) {
                return Err(kv_application::RerankError::ModelNotCached {
                    model: model.as_str().to_owned(),
                });
            }
            *session = Some(build_rerank_session(fastembed_model, &cache_dir)?);
        }

        let model = session.as_mut().ok_or_else(|| {
            kv_application::RerankError::Storage(Box::new(std::io::Error::other(
                "re-ranking session is unavailable",
            )))
        })?;
        let results = model
            .rerank(query, documents, false, None)
            .map_err(|error| kv_application::RerankError::Storage(Box::new(error)))?;

        let mut scores = vec![0.0; documents.len()];
        for result in results {
            let index = result.index;
            if let Some(score) = scores.get_mut(index) {
                *score = f64::from(result.score);
            }
        }

        Ok(scores)
    }
}

impl FastembedReranker {
    fn effective_cache_dir(&self) -> PathBuf {
        if let Some(cache_dir) = &self.cache_dir {
            return cache_dir.clone();
        }
        if let Ok(hf_home) = std::env::var("HF_HOME") {
            return PathBuf::from(hf_home);
        }
        if let Ok(cache_dir) = std::env::var("FASTEMBED_CACHE_DIR") {
            return PathBuf::from(cache_dir);
        }
        PathBuf::from(".fastembed_cache")
    }
}

/// Maps a domain re-ranker model name to a fastembed model.
fn resolve_reranker_model(
    model: &RerankerModel,
) -> Result<FastembedRerankerModel, kv_application::RerankError> {
    friendly_reranker_model(model.as_str())
        .or_else(|| model.as_str().parse::<FastembedRerankerModel>().ok())
        .ok_or_else(|| kv_application::RerankError::UnsupportedModel {
            model: model.as_str().to_owned(),
        })
}

/// Maps the friendly re-ranker names `mdsearch` documents to fastembed.
fn friendly_reranker_model(name: &str) -> Option<FastembedRerankerModel> {
    match name {
        "bge-reranker-base" => Some(FastembedRerankerModel::BGERerankerBase),
        "bge-reranker-v2-m3" => Some(FastembedRerankerModel::BGERerankerV2M3),
        "jina-reranker-v1-turbo-en" => Some(FastembedRerankerModel::JINARerankerV1TurboEn),
        "jina-reranker-v2-base-multilingual" => {
            Some(FastembedRerankerModel::JINARerankerV2BaseMultiligual)
        }
        _ => None,
    }
}

/// Builds a `TextRerank` session for the model, downloading assets if needed.
fn build_rerank_session(
    model: FastembedRerankerModel,
    cache_dir: &Path,
) -> Result<TextRerank, kv_application::RerankError> {
    let options = RerankInitOptions::new(model)
        .with_cache_dir(cache_dir.to_owned())
        .with_show_download_progress(false);
    TextRerank::try_new(options).map_err(|error| match error {
        fastembed::Error::ModelRetrieval { file, source } => {
            kv_application::RerankError::DownloadFailed {
                model: file,
                source,
            }
        }
        other => kv_application::RerankError::Storage(Box::new(other)),
    })
}

/// Returns whether the model's primary file is present in the local cache.
///
/// The check mirrors the hf-hub cache layout fastembed uses, matching the
/// embedding adapter's availability check.
fn model_is_cached(cache_dir: &Path, model: &FastembedRerankerModel) -> bool {
    let info = fastembed::TextRerank::get_model_info(model);
    let folder = cache_dir.join(info.model_code.replace('/', "--"));
    let commit_path = folder.join("refs").join("main");
    let Ok(commit) = std::fs::read_to_string(commit_path) else {
        return false;
    };
    let snapshot = folder.join("snapshots").join(commit.trim());
    snapshot.join(&info.model_file).exists()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use fastembed::RerankerModel as FastembedRerankerModel;
    use kv_application::Reranker;
    use kv_domain::RerankerModel;
    use tempfile::tempdir;

    use super::friendly_reranker_model;
    use super::model_is_cached;
    use super::resolve_reranker_model;

    /// Covers: REQ-011 — the default friendly name resolves to fastembed.
    #[test]
    fn friendly_reranker_names_resolve_to_fastembed_models() {
        assert_eq!(
            friendly_reranker_model("bge-reranker-base"),
            Some(FastembedRerankerModel::BGERerankerBase)
        );
        assert_eq!(
            friendly_reranker_model("bge-reranker-v2-m3"),
            Some(FastembedRerankerModel::BGERerankerV2M3)
        );
    }

    /// Covers: REQ-011 — unknown names are rejected.
    #[test]
    fn unknown_reranker_names_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let model = RerankerModel::try_new("bogus")?;

        assert!(matches!(
            resolve_reranker_model(&model),
            Err(kv_application::RerankError::UnsupportedModel { .. })
        ));

        Ok(())
    }

    /// Covers: REQ-011 FR-006 — an uncached re-ranker fails without download.
    #[test]
    fn uncached_reranker_fails_without_download() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = RerankerModel::try_new("bge-reranker-base")?;
        let reranker = super::FastembedReranker::new(Some(cache_dir.path().to_owned()));

        assert!(matches!(
            reranker.ensure_available(&model, false),
            Err(kv_application::RerankError::ModelNotCached { .. })
        ));

        Ok(())
    }

    /// Covers: DES-011 — the availability check matches the hf-hub cache layout.
    #[test]
    fn reranker_availability_check_matches_hf_hub_cache_layout()
    -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let info = fastembed::TextRerank::get_model_info(&FastembedRerankerModel::BGERerankerBase);
        let folder = cache_dir.path().join(info.model_code.replace('/', "--"));
        let snapshot = folder.join("snapshots").join("abcdef");
        fs::create_dir_all(&snapshot)?;
        fs::create_dir_all(folder.join("refs"))?;
        fs::write(folder.join("refs").join("main"), "abcdef")?;
        let model_file = snapshot.join(&info.model_file);
        if let Some(parent) = model_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(model_file, b"fake onnx")?;

        assert!(model_is_cached(
            cache_dir.path(),
            &FastembedRerankerModel::BGERerankerBase
        ));

        Ok(())
    }

    /// Covers: DES-011 — a missing commit pointer means the model is not cached.
    #[test]
    fn missing_commit_pointer_means_not_cached() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let info = fastembed::TextRerank::get_model_info(&FastembedRerankerModel::BGERerankerBase);
        let folder = cache_dir.path().join(info.model_code.replace('/', "--"));
        fs::create_dir_all(&folder)?;

        assert!(!model_is_cached(
            cache_dir.path(),
            &FastembedRerankerModel::BGERerankerBase
        ));

        Ok(())
    }
}
