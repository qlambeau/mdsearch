use std::cell::RefCell;
use std::path::{Path, PathBuf};

use fastembed::{EmbeddingModel as FastembedModel, TextEmbedding, TextInitOptions};
use kv_application::EmbeddingGenerator;
use kv_domain::{Embedding, EmbeddingModel};

use crate::marker;

/// Generates text embeddings locally with `fastembed`.
pub struct FastembedGenerator {
    session: RefCell<Option<TextEmbedding>>,
    cache_dir: PathBuf,
}

impl FastembedGenerator {
    /// Creates a generator that loads models from the given cache directory.
    ///
    /// The cache directory is resolved once by the caller (ADR-012): the
    /// embedding adapter no longer inspects environment variables.
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
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

        if marker::marker_exists(&self.cache_dir, model.as_str()) {
            return Ok(());
        }

        if !download {
            return Err(kv_application::EmbeddingError::ModelNotCached {
                model: model.as_str().to_owned(),
            });
        }

        let session = build_session(fastembed_model, &self.cache_dir)?;
        marker::write_marker(&self.cache_dir, model.as_str())
            .map_err(|source| kv_application::EmbeddingError::Storage(Box::new(source)))?;
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
            if !marker::marker_exists(&self.cache_dir, model.as_str()) {
                return Err(kv_application::EmbeddingError::ModelNotCached {
                    model: model.as_str().to_owned(),
                });
            }
            *session = Some(build_session(fastembed_model, &self.cache_dir)?);
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::friendly_model;
    use super::resolve_model;
    use crate::marker;
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

    /// Covers: REQ-009 — an uncached model fails before any collection work.
    #[test]
    fn uncached_model_fails_without_download() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = EmbeddingModel::try_new("all-MiniLM-L6-v2")?;
        let generator = super::FastembedGenerator::new(cache_dir.path().to_owned());

        assert!(matches!(
            generator.ensure_available(&model, false),
            Err(kv_application::EmbeddingError::ModelNotCached { .. })
        ));

        Ok(())
    }

    /// Covers: REQ-017 FR-004/FR-005 — a completion marker alone makes the
    /// model available without any hf-hub layout.
    #[test]
    fn downloaded_model_is_recognized_via_marker() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = EmbeddingModel::try_new("all-MiniLM-L6-v2")?;
        marker::write_marker(cache_dir.path(), model.as_str())?;
        let generator = super::FastembedGenerator::new(cache_dir.path().to_owned());

        assert!(generator.ensure_available(&model, false).is_ok());

        Ok(())
    }

    /// Covers: REQ-017 FR-004 — an existing marker avoids re-downloading.
    #[test]
    fn existing_marker_avoids_redownload() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;
        let model = EmbeddingModel::try_new("all-MiniLM-L6-v2")?;
        marker::write_marker(cache_dir.path(), model.as_str())?;
        let generator = super::FastembedGenerator::new(cache_dir.path().to_owned());

        assert!(generator.ensure_available(&model, true).is_ok());

        Ok(())
    }

    /// Covers: REQ-017 FR-004 — the marker round-trips through the cache
    /// directory.
    #[test]
    fn marker_round_trips_through_cache_directory() -> Result<(), Box<dyn std::error::Error>> {
        let cache_dir = tempdir()?;

        marker::write_marker(cache_dir.path(), "all-MiniLM-L6-v2")?;

        assert!(marker::marker_exists(cache_dir.path(), "all-MiniLM-L6-v2"));

        Ok(())
    }
}
