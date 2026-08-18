use kv_domain::{Embedding, EmbeddingModel};

use crate::EmbeddingError;

/// Generates local text embeddings for the semantic index.
pub trait EmbeddingGenerator {
    /// Ensures the model's assets are available locally, downloading them when
    /// `download` is set.
    ///
    /// # Errors
    ///
    /// Returns an unsupported-model error when the model is not supported, a
    /// not-cached error when the model is absent and no download was requested,
    /// or a download-failed error when fetching the assets fails.
    fn ensure_available(
        &self,
        model: &EmbeddingModel,
        download: bool,
    ) -> Result<(), EmbeddingError>;

    /// Generates one embedding per input text for the given model.
    ///
    /// # Errors
    ///
    /// Returns a storage error when generation fails.
    fn embed(
        &self,
        model: &EmbeddingModel,
        texts: &[&str],
    ) -> Result<Vec<Embedding>, EmbeddingError>;
}
