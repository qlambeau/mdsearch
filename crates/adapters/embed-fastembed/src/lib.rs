#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `fastembed` adapter for the `mdsearch` embedding generator port.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel as FastembedModel, TextEmbedding, TextInitOptions};
use kv_application::EmbeddingGenerator;
use kv_domain::{Embedding, EmbeddingModel};

/// Generates text embeddings locally with `fastembed`.
pub struct FastembedGenerator {
    session: RefCell<Option<TextEmbedding>>,
    cache_dir: Option<PathBuf>,
}

impl FastembedGenerator {
    /// Creates a generator that loads models from fastembed's cache.
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

impl EmbeddingGenerator for FastembedGenerator {
    fn ensure_available(
        &self,
        model: &EmbeddingModel,
        download: bool,
    ) -> Result<(), kv_application::EmbeddingError> {
        let fastembed_model = resolve_model(model)?;
        let cache_dir = self.effective_cache_dir();

        if model_is_cached(&cache_dir, &fastembed_model) {
            return Ok(());
        }

        if !download {
            return Err(kv_application::EmbeddingError::ModelNotCached {
                model: model.as_str().to_owned(),
            });
        }

        let session = build_session(fastembed_model, &cache_dir)?;
        *self.session.borrow_mut() = Some(session);

        Ok(())
    }

    fn embed(
        &self,
        model: &EmbeddingModel,
        texts: &[&str],
    ) -> Result<Vec<Embedding>, kv_application::EmbeddingError> {
        let fastembed_model = resolve_model(model)?;

        let mut session = self.session.borrow_mut();
        if session.is_none() {
            let cache_dir = self.effective_cache_dir();
            if !model_is_cached(&cache_dir, &fastembed_model) {
                return Err(kv_application::EmbeddingError::ModelNotCached {
                    model: model.as_str().to_owned(),
                });
            }
            *session = Some(build_session(fastembed_model, &cache_dir)?);
        }

        let model = session.as_mut().ok_or_else(|| {
            kv_application::EmbeddingError::Storage(Box::new(std::io::Error::other(
                "embedding session is unavailable",
            )))
        })?;
        let vectors = model
            .embed(texts, None)
            .map_err(|error| kv_application::EmbeddingError::Storage(Box::new(error)))?;

        Ok(vectors.into_iter().map(Embedding::new).collect())
    }
}

impl FastembedGenerator {
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

/// Maps a domain model name to a fastembed model, delegating to fastembed's
/// supported model set and its own name parsing.
fn resolve_model(model: &EmbeddingModel) -> Result<FastembedModel, kv_application::EmbeddingError> {
    friendly_model(model.as_str())
        .or_else(|| model.as_str().parse::<FastembedModel>().ok())
        .ok_or_else(|| kv_application::EmbeddingError::UnsupportedModel {
            model: model.as_str().to_owned(),
        })
}

/// Maps the friendly model names `mdsearch` documents to fastembed models.
fn friendly_model(name: &str) -> Option<FastembedModel> {
    match name {
        "all-MiniLM-L6-v2" => Some(FastembedModel::AllMiniLML6V2),
        "bge-small-en-v1.5" => Some(FastembedModel::BGESmallENV15),
        "bge-base-en-v1.5" => Some(FastembedModel::BGEBaseENV15),
        "bge-large-en-v1.5" => Some(FastembedModel::BGELargeENV15),
        "multilingual-e5-small" => Some(FastembedModel::MultilingualE5Small),
        "multilingual-e5-base" => Some(FastembedModel::MultilingualE5Base),
        "multilingual-e5-large" => Some(FastembedModel::MultilingualE5Large),
        _ => None,
    }
}

/// Builds a `TextEmbedding` session for the model, downloading assets if
/// needed.
fn build_session(
    model: FastembedModel,
    cache_dir: &Path,
) -> Result<TextEmbedding, kv_application::EmbeddingError> {
    let options = TextInitOptions::new(model)
        .with_cache_dir(cache_dir.to_owned())
        .with_show_download_progress(false);
    TextEmbedding::try_new(options).map_err(|error| match error {
        fastembed::Error::ModelRetrieval { file, source } => {
            kv_application::EmbeddingError::DownloadFailed {
                model: file,
                source,
            }
        }
        other => kv_application::EmbeddingError::Storage(Box::new(other)),
    })
}

/// Returns whether the model's primary file is present in the local cache.
///
/// The check mirrors the hf-hub cache layout fastembed uses: the repo folder
/// holds a `refs/main` pointer to a commit hash, and the model file lives under
/// `snapshots/{commit}/{file}`.
fn model_is_cached(cache_dir: &Path, model: &FastembedModel) -> bool {
    let Some(info) = fastembed::TextEmbedding::get_model_info(model).ok() else {
        return false;
    };
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

    use tempfile::tempdir;

    use super::{friendly_model, model_is_cached, resolve_model};
    use fastembed::EmbeddingModel as FastembedModel;
    use kv_application::EmbeddingGenerator;
    use kv_domain::EmbeddingModel;

    /// Covers: REQ-010 — the default friendly name resolves to fastembed.
    #[test]
    fn friendly_names_resolve_to_fastembed_models() {
        assert_eq!(
            friendly_model("all-MiniLM-L6-v2"),
            Some(FastembedModel::AllMiniLML6V2)
        );
        assert_eq!(
            friendly_model("bge-small-en-v1.5"),
            Some(FastembedModel::BGESmallENV15)
        );
    }

    /// Covers: REQ-010 — unknown names are rejected.
    #[test]
    fn unknown_names_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let model = EmbeddingModel::try_new("bogus")?;

        assert!(matches!(
            resolve_model(&model),
            Err(kv_application::EmbeddingError::UnsupportedModel { .. })
        ));

        Ok(())
    }

    /// Covers: the approved default model is 384-dimensional.
    #[test]
    fn default_model_is_384_dimensional() -> Result<(), Box<dyn std::error::Error>> {
        let info = fastembed::TextEmbedding::get_model_info(&FastembedModel::AllMiniLML6V2)?;

        assert_eq!(info.dim, 384);

        Ok(())
    }

    /// Covers: REQ-009 — an uncached model fails before any collection work.
    #[test]
    fn uncached_model_fails_without_download() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = EmbeddingModel::try_new("all-MiniLM-L6-v2")?;
        let generator = super::FastembedGenerator::new(Some(cache_dir.path().to_owned()));

        assert!(matches!(
            generator.ensure_available(&model, false),
            Err(kv_application::EmbeddingError::ModelNotCached { .. })
        ));

        Ok(())
    }

    /// Covers: DES-010 — the availability check matches the hf-hub cache layout.
    #[test]
    fn availability_check_matches_hf_hub_cache_layout() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let info = fastembed::TextEmbedding::get_model_info(&FastembedModel::AllMiniLML6V2)?;
        let folder = cache_dir.path().join(info.model_code.replace('/', "--"));
        let snapshot = folder.join("snapshots").join("abcdef");
        fs::create_dir_all(&snapshot)?;
        fs::create_dir_all(folder.join("refs"))?;
        fs::write(folder.join("refs").join("main"), "abcdef")?;
        fs::write(snapshot.join(&info.model_file), b"fake onnx")?;

        assert!(model_is_cached(
            cache_dir.path(),
            &FastembedModel::AllMiniLML6V2
        ));

        Ok(())
    }
}
