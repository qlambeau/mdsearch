use std::cell::RefCell;
use std::path::{Path, PathBuf};

use fastembed::{RerankInitOptions, RerankerModel as FastembedRerankerModel, TextRerank};
use kv_application::Reranker;
use kv_domain::RerankerModel;

use crate::marker;

/// Re-scores candidate documents locally with `fastembed`'s `TextRerank`.
pub struct FastembedReranker {
    session: RefCell<Option<TextRerank>>,
    cache_dir: PathBuf,
}

impl FastembedReranker {
    /// Creates a re-ranker that loads models from the given cache directory.
    ///
    /// The cache directory is resolved once by the caller (ADR-012): the
    /// re-ranker adapter no longer inspects environment variables.
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
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

        if marker::marker_exists(&self.cache_dir, model.as_str()) {
            return Ok(());
        }

        if !download {
            return Err(kv_application::RerankError::ModelNotCached {
                model: model.as_str().to_owned(),
            });
        }

        let session = build_rerank_session(fastembed_model, &self.cache_dir)?;
        marker::write_marker(&self.cache_dir, model.as_str())
            .map_err(|source| kv_application::RerankError::Storage(Box::new(source)))?;
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
            if !marker::marker_exists(&self.cache_dir, model.as_str()) {
                return Err(kv_application::RerankError::ModelNotCached {
                    model: model.as_str().to_owned(),
                });
            }
            *session = Some(build_rerank_session(fastembed_model, &self.cache_dir)?);
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

#[cfg(test)]
mod tests {
    use fastembed::RerankerModel as FastembedRerankerModel;
    use kv_application::Reranker;
    use kv_domain::RerankerModel;
    use tempfile::tempdir;

    use super::friendly_reranker_model;
    use super::resolve_reranker_model;
    use crate::marker;

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
        let reranker = super::FastembedReranker::new(cache_dir.path().to_owned());

        assert!(matches!(
            reranker.ensure_available(&model, false),
            Err(kv_application::RerankError::ModelNotCached { .. })
        ));

        Ok(())
    }

    /// Covers: REQ-017 FR-007 — a completion marker alone makes the re-ranker
    /// available without any hf-hub layout.
    #[test]
    fn downloaded_reranker_is_recognized_via_marker() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = RerankerModel::try_new("bge-reranker-base")?;
        marker::write_marker(cache_dir.path(), model.as_str())?;
        let reranker = super::FastembedReranker::new(cache_dir.path().to_owned());

        assert!(reranker.ensure_available(&model, false).is_ok());

        Ok(())
    }

    /// Covers: REQ-017 FR-007 — an existing re-ranker marker avoids
    /// re-downloading.
    #[test]
    fn existing_reranker_marker_avoids_redownload() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = RerankerModel::try_new("bge-reranker-base")?;
        marker::write_marker(cache_dir.path(), model.as_str())?;
        let reranker = super::FastembedReranker::new(cache_dir.path().to_owned());

        assert!(reranker.ensure_available(&model, true).is_ok());

        Ok(())
    }
}
