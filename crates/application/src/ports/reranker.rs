use kv_domain::RerankerModel;

use crate::RerankError;

/// Re-scores candidate documents against a query with a local cross-encoder.
pub trait Reranker {
    /// Ensures the model's assets are available locally, downloading them when
    /// `download` is set.
    ///
    /// # Errors
    ///
    /// Returns an unsupported-model error when the model is not supported, a
    /// not-cached error when the model is absent and no download was requested,
    /// or a download-failed error when fetching the assets fails.
    fn ensure_available(&self, model: &RerankerModel, download: bool) -> Result<(), RerankError>;

    /// Re-scores `documents` against `query`, returning one score per document
    /// in input order. Higher scores indicate stronger relevance.
    ///
    /// # Errors
    ///
    /// Returns a storage error when re-ranking fails.
    fn rerank(
        &self,
        model: &RerankerModel,
        query: &str,
        documents: &[&str],
    ) -> Result<Vec<f64>, RerankError>;
}
