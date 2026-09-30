use std::path::PathBuf;

use kv_application::{EmbeddingError, EmbeddingGenerator};
use kv_domain::{Embedding, EmbeddingModel};
use kv_embed_fastembed::FastembedGenerator;

/// Optional model collaborator: lexical-only operations never need a cache path.
pub(crate) struct ModelGenerator {
    generator: Option<FastembedGenerator>,
}

impl ModelGenerator {
    pub(crate) fn new(cache: Option<PathBuf>) -> Self {
        Self {
            generator: cache.map(FastembedGenerator::new),
        }
    }

    fn generator(&self) -> Result<&FastembedGenerator, EmbeddingError> {
        self.generator
            .as_ref()
            .ok_or_else(|| EmbeddingError::Storage(Box::new(crate::AppError::HomeUnavailable)))
    }
}

impl EmbeddingGenerator for ModelGenerator {
    fn ensure_available(
        &self,
        model: &EmbeddingModel,
        download: bool,
    ) -> Result<(), EmbeddingError> {
        self.generator()?.ensure_available(model, download)
    }

    fn embed(
        &self,
        model: &EmbeddingModel,
        texts: &[&str],
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        self.generator()?.embed(model, texts)
    }
}

/// Resolves missing cache prerequisites only when reranking is requested.
pub(crate) struct ModelReranker {
    reranker: Option<kv_embed_fastembed::FastembedReranker>,
}

impl ModelReranker {
    pub(crate) fn new(cache: Option<PathBuf>) -> Self {
        Self {
            reranker: cache.map(kv_embed_fastembed::FastembedReranker::new),
        }
    }

    fn reranker(
        &self,
    ) -> Result<&kv_embed_fastembed::FastembedReranker, kv_application::RerankError> {
        self.reranker.as_ref().ok_or_else(|| {
            kv_application::RerankError::Storage(Box::new(crate::AppError::HomeUnavailable))
        })
    }
}

impl kv_application::Reranker for ModelReranker {
    fn ensure_available(
        &self,
        model: &kv_domain::RerankerModel,
        download: bool,
    ) -> Result<(), kv_application::RerankError> {
        self.reranker()?.ensure_available(model, download)
    }

    fn rerank(
        &self,
        model: &kv_domain::RerankerModel,
        query: &str,
        documents: &[&str],
    ) -> Result<Vec<f64>, kv_application::RerankError> {
        self.reranker()?.rerank(model, query, documents)
    }
}
