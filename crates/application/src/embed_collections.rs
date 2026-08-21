use kv_domain::{
    CollectionName, Embedding, EmbeddingModel, FileId, RerankerModel, SemanticPassage, Timestamp,
};

use crate::{
    Clock, EmbedError, EmbeddingGenerator, RerankError, Reranker, SemanticIndexStore,
    SemanticIndexStoreError,
};

/// The scope of an embed operation.
#[derive(Clone, Copy, Debug)]
pub enum EmbedScope<'a> {
    /// Embed every eligible collection.
    All,
    /// Embed a single named collection.
    Collection(&'a CollectionName),
}

/// Why a collection was not embedded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkipReason {
    /// The collection has no stored files.
    NoFiles,
    /// The collection's lexical index has never been built.
    LexicalNotBuilt,
}

/// A progress event emitted during an embedding run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbedProgress {
    /// Per-file progress within a collection's embedding phase.
    Files {
        /// The collection being embedded.
        collection: CollectionName,
        /// Files whose passages have been embedded so far.
        completed_files: usize,
        /// Files with passages in this collection's embedding set.
        total_files: usize,
    },
    /// The collection's vectors are being written to the index.
    Writing {
        /// The collection being written.
        collection: CollectionName,
    },
}

/// The outcome for one processed collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbedOutcome {
    /// The collection's semantic index was built or rebuilt.
    Embedded {
        /// The collection name.
        collection: CollectionName,
        /// The number of passages embedded.
        passage_count: usize,
    },
    /// The collection's semantic index is already current.
    AlreadyCurrent {
        /// The collection name.
        collection: CollectionName,
    },
    /// The collection was skipped without being embedded.
    Skipped {
        /// The collection name.
        collection: CollectionName,
        /// The reason the collection was skipped.
        reason: SkipReason,
    },
    /// The collection failed to embed; processing continued.
    Failed {
        /// The collection name.
        collection: CollectionName,
        /// A human-readable description of the failure.
        message: String,
    },
}

impl EmbedOutcome {
    /// Returns the collection this outcome concerns.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        match self {
            Self::Embedded { collection, .. }
            | Self::AlreadyCurrent { collection }
            | Self::Skipped { collection, .. }
            | Self::Failed { collection, .. } => collection,
        }
    }

    /// Returns whether this outcome is a failure.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }
}

/// The per-collection outcomes of one embed run.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EmbedReport {
    outcomes: Vec<EmbedOutcome>,
}

impl EmbedReport {
    /// Creates an empty embed report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            outcomes: Vec::new(),
        }
    }

    /// Adds an outcome to the report.
    pub fn push(&mut self, outcome: EmbedOutcome) {
        self.outcomes.push(outcome);
    }

    /// Returns the per-collection outcomes in processing order.
    #[must_use]
    pub fn outcomes(&self) -> &[EmbedOutcome] {
        &self.outcomes
    }

    /// Returns whether any collection failed.
    #[must_use]
    pub fn any_failed(&self) -> bool {
        self.outcomes.iter().any(EmbedOutcome::is_failed)
    }
}

/// The default embedding model when none is recorded or supplied.
const DEFAULT_MODEL: &str = "all-MiniLM-L6-v2";

/// Orchestrates building the semantic index for collections.
pub struct EmbedCollections<G, S, C, R> {
    generator: G,
    store: S,
    clock: C,
    reranker: R,
}

impl<G, S, C, R> EmbedCollections<G, S, C, R>
where
    G: EmbeddingGenerator,
    S: SemanticIndexStore,
    C: Clock,
    R: Reranker,
{
    /// Creates an embed-collections use case with its generator, store, clock,
    /// and re-ranker ports.
    #[must_use]
    pub const fn new(generator: G, store: S, clock: C, reranker: R) -> Self {
        Self {
            generator,
            store,
            clock,
            reranker,
        }
    }

    /// Builds the semantic index for the collections selected by `scope` and
    /// optionally provisions the re-ranker.
    ///
    /// Progress events (`EmbedProgress`) are reported through `progress`
    /// during the embedding phase; the vector write phase is announced with a
    /// single `Writing` event per collection.
    ///
    /// # Errors
    ///
    /// Returns an error when the model or re-ranker is unsupported or
    /// unavailable, when a targeted collection does not exist or lacks a built
    /// lexical index, or when the clock or store fails outside a per-collection
    /// rebuild.
    pub fn execute(
        &mut self,
        scope: EmbedScope<'_>,
        model: Option<&EmbeddingModel>,
        reranker: Option<&RerankerModel>,
        download: bool,
        progress: &mut dyn FnMut(EmbedProgress),
    ) -> Result<EmbedReport, EmbedError> {
        if let Some(reranker_model) = reranker {
            self.provision_reranker(reranker_model, download)?;
        }

        let recorded = self.store.global_model()?;
        let effective = match model.or(recorded.as_ref()) {
            Some(model) => model.clone(),
            None => default_model()?,
        };

        self.generator.ensure_available(&effective, download)?;

        if recorded.as_ref() != Some(&effective) {
            self.store.set_global_model(&effective)?;
        }
        let model_changed = model.is_some_and(|given| recorded.as_ref() != Some(given));

        let mut report = EmbedReport::new();
        let now = self.clock.now()?;
        let mut process = self.resolve_targets(scope, &mut report)?;

        if model_changed {
            for collection in self.store.embedded_collections()? {
                if !process.iter().any(|name| name == &collection) {
                    process.push(collection);
                }
            }
        }

        for collection in process {
            let outcome = self.embed_collection(&collection, &effective, now, progress);
            match outcome {
                Ok(outcome) => report.push(outcome),
                Err(message) => report.push(EmbedOutcome::Failed {
                    collection,
                    message,
                }),
            }
        }

        Ok(report)
    }

    fn provision_reranker(
        &mut self,
        reranker_model: &RerankerModel,
        download: bool,
    ) -> Result<(), EmbedError> {
        match self.reranker.ensure_available(reranker_model, download) {
            Ok(()) => {}
            Err(RerankError::UnsupportedModel { model }) => {
                return Err(EmbedError::Reranker(RerankError::UnsupportedModel {
                    model,
                }));
            }
            Err(RerankError::ModelNotCached { model }) => {
                return Err(EmbedError::Reranker(RerankError::ModelNotCached { model }));
            }
            Err(RerankError::DownloadFailed { model, source }) => {
                return Err(EmbedError::Reranker(RerankError::DownloadFailed {
                    model,
                    source,
                }));
            }
            Err(RerankError::Storage(source)) => {
                return Err(EmbedError::Reranker(RerankError::Storage(source)));
            }
        }
        let recorded = self.store.reranker_model()?;
        if recorded.as_ref() != Some(reranker_model) {
            self.store.set_reranker_model(reranker_model)?;
        }
        Ok(())
    }

    fn resolve_targets(
        &self,
        scope: EmbedScope<'_>,
        report: &mut EmbedReport,
    ) -> Result<Vec<CollectionName>, EmbedError> {
        match scope {
            EmbedScope::All => {
                let mut process = Vec::new();
                for target in self.store.targets()? {
                    if !target.has_files() {
                        report.push(EmbedOutcome::Skipped {
                            collection: target.collection().clone(),
                            reason: SkipReason::NoFiles,
                        });
                    } else if !target.lexical_built() {
                        report.push(EmbedOutcome::Skipped {
                            collection: target.collection().clone(),
                            reason: SkipReason::LexicalNotBuilt,
                        });
                    } else {
                        process.push(target.collection().clone());
                    }
                }
                Ok(process)
            }
            EmbedScope::Collection(collection) => {
                let target = self
                    .store
                    .resolve(collection)
                    .map_err(|error| match error {
                        SemanticIndexStoreError::CollectionNotFound => {
                            EmbedError::CollectionNotFound
                        }
                        SemanticIndexStoreError::Storage(source) => {
                            EmbedError::Store(SemanticIndexStoreError::Storage(source))
                        }
                    })?;
                if !target.has_files() {
                    report.push(EmbedOutcome::Skipped {
                        collection: collection.clone(),
                        reason: SkipReason::NoFiles,
                    });
                    Ok(Vec::new())
                } else if !target.lexical_built() {
                    Err(EmbedError::IndexNotBuilt)
                } else {
                    Ok(vec![collection.clone()])
                }
            }
        }
    }

    fn embed_collection(
        &mut self,
        collection: &CollectionName,
        model: &EmbeddingModel,
        now: Timestamp,
        progress: &mut dyn FnMut(EmbedProgress),
    ) -> Result<EmbedOutcome, String> {
        let fingerprint = self
            .store
            .file_set_fingerprint(collection)
            .map_err(|error| semantic_error_message(&error))?;
        let status = self
            .store
            .status(collection)
            .map_err(|error| semantic_error_message(&error))?;

        let needs_rebuild = match &status {
            None => true,
            Some(status) => {
                status.file_set_fingerprint() != &fingerprint || status.model() != model
            }
        };

        if !needs_rebuild {
            return Ok(EmbedOutcome::AlreadyCurrent {
                collection: collection.clone(),
            });
        }

        let passages = self
            .store
            .passages(collection)
            .map_err(|error| semantic_error_message(&error))?;
        let mut pairs = Vec::with_capacity(passages.len());
        let mut groups: Vec<(FileId, Vec<SemanticPassage>)> = Vec::new();
        for passage in passages {
            match groups.iter_mut().find(|(file, _)| *file == passage.file()) {
                Some((_, file_passages)) => file_passages.push(passage),
                None => groups.push((passage.file(), vec![passage])),
            }
        }
        let total_files = groups.len();
        for (index, (_, file_passages)) in groups.iter().enumerate() {
            let texts = file_passages
                .iter()
                .map(SemanticPassage::text)
                .collect::<Vec<_>>();
            let vectors = self
                .generator
                .embed(model, &texts)
                .map_err(|error| error.to_string())?;
            let file_pairs = build_pairs(file_passages.clone(), vectors)
                .ok_or_else(|| "embedding count does not match passage count".to_owned())?;
            pairs.extend(file_pairs);
            progress(EmbedProgress::Files {
                collection: collection.clone(),
                completed_files: index + 1,
                total_files,
            });
        }
        progress(EmbedProgress::Writing {
            collection: collection.clone(),
        });

        let passage_count = match pairs.first() {
            Some((_, first)) => {
                self.store
                    .ensure_dimension(first.as_slice().len())
                    .map_err(|error| semantic_error_message(&error))?;
                self.store
                    .rebuild(collection, model, now, &pairs)
                    .map_err(|error| semantic_error_message(&error))?
            }
            None => self
                .store
                .rebuild(collection, model, now, &pairs)
                .map_err(|error| semantic_error_message(&error))?,
        };

        Ok(EmbedOutcome::Embedded {
            collection: collection.clone(),
            passage_count,
        })
    }
}

fn semantic_error_message(error: &SemanticIndexStoreError) -> String {
    error.to_string()
}

/// Returns the default embedding model, which is a compile-time constant.
fn default_model() -> Result<EmbeddingModel, EmbedError> {
    EmbeddingModel::try_new(DEFAULT_MODEL)
        .map_err(|error| EmbedError::Generator(crate::EmbeddingError::Storage(Box::new(error))))
}

/// Zips passages and their embeddings, requiring equal lengths.
fn build_pairs(
    passages: Vec<SemanticPassage>,
    vectors: Vec<Embedding>,
) -> Option<Vec<(SemanticPassage, Embedding)>> {
    if passages.len() != vectors.len() {
        return None;
    }
    Some(passages.into_iter().zip(vectors).collect())
}
