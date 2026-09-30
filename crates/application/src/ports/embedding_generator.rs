use kv_domain::{Embedding, EmbeddingModel};

use crate::EmbeddingError;

/// Describes a supported local model and whether its assets are cached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelAvailability {
    name: String,
    available: bool,
}

impl ModelAvailability {
    /// Creates one supported-model availability entry.
    #[must_use]
    pub const fn new(name: String, available: bool) -> Self {
        Self { name, available }
    }

    /// Returns the canonical model name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns whether the model assets are cached locally.
    #[must_use]
    pub const fn available(&self) -> bool {
        self.available
    }
}

/// Generates local text embeddings for the semantic index.
pub trait EmbeddingGenerator {
    /// Returns supported embedding models and local cache state.
    fn models(&self) -> Vec<ModelAvailability> {
        Vec::new()
    }
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
