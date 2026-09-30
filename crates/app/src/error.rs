use thiserror::Error;

use kv_application::{
    AddFilesError, ClockError, CollectionStoreError, CreateCollectionError, DestroyCollectionError,
    EmbedError, FileSystemError, GetFileError, GraphStoreError, HybridError, IndexStatusError,
    ListCollectionsError, SearchError, SemanticIndexStoreError, UpdateCollectionError,
};
use kv_domain::{CollectionNameError, EmbeddingModelError, RerankerModelError};

/// Describes a user-visible failure from the `mdsearch` CLI.
#[derive(Debug, Error)]
pub enum AppError {
    /// A complete update report whose failed outcomes require exit 1.
    #[error("{0}")]
    UpdateReportFailed(String),
    /// The command needs a default path but the home directory is unavailable.
    #[error("home directory is unavailable")]
    HomeUnavailable,
    /// Clap rejected the command-line arguments.
    #[error(transparent)]
    Arguments(#[from] clap::Error),
    /// The collection name is invalid.
    #[error(transparent)]
    InvalidName(#[from] CollectionNameError),
    /// The application use case failed.
    #[error(transparent)]
    CreateCollection(#[from] CreateCollectionError),
    /// The list-collections use case failed.
    #[error(transparent)]
    ListCollections(#[from] ListCollectionsError),
    /// The destroy-collection use case failed.
    #[error(transparent)]
    DestroyCollection(#[from] DestroyCollectionError),
    /// The add-files use case failed.
    #[error(transparent)]
    AddFiles(#[from] AddFilesError),
    /// The update-collection use case failed.
    #[error(transparent)]
    UpdateCollection(#[from] UpdateCollectionError),
    /// The index-status use case failed.
    #[error(transparent)]
    IndexStatus(#[from] IndexStatusError),
    /// The lexical-search use case failed.
    #[error(transparent)]
    Search(#[from] SearchError),
    /// The get-file use case failed.
    #[error(transparent)]
    GetFile(#[from] GetFileError),
    /// The embed-collections use case failed.
    #[error(transparent)]
    Embed(#[from] EmbedError),
    /// The semantic-index store operation failed.
    #[error(transparent)]
    SemanticIndexStore(#[from] SemanticIndexStoreError),
    /// The hybrid-search use case failed.
    #[error(transparent)]
    Hybrid(#[from] HybridError),
    /// A hybrid prerequisite failed, with an actionable recovery command.
    #[error("{source}; recover with: {recovery}")]
    HybridPrerequisite {
        /// The typed retrieval failure and its cause chain.
        #[source]
        source: HybridError,
        /// A command using the selected database and final grammar.
        recovery: String,
    },
    /// The graph query failed.
    #[error(transparent)]
    Graph(#[from] GraphStoreError),
    /// The graph context query failed.
    #[error("context query failed: {0}")]
    GraphQuery(String),
    /// Model validation or provisioning failed before database mutation.
    #[error("{0}")]
    Model(String),
    /// The embedding model name is invalid.
    #[error(transparent)]
    InvalidEmbeddingModel(#[from] EmbeddingModelError),
    /// The re-ranker model name is invalid.
    #[error(transparent)]
    InvalidRerankerModel(#[from] RerankerModelError),
    /// The embed-collections use case completed with per-collection failures.
    #[error("{0}")]
    EmbedPartial(String),
    /// The retrieved file content is not valid UTF-8.
    #[error("file content is not valid UTF-8")]
    NonUtf8Content,
    /// The database could not be opened or accessed.
    #[error(transparent)]
    CollectionStore(#[from] CollectionStoreError),
    /// One or more collection updates failed after all collections were attempted.
    #[error("update completed with failures:\n{0}")]
    UpdateAllFailed(String),
    /// The collection has no registered filesystem sources.
    #[error("collection {0} has no registered sources; configure sources before updating")]
    NoRegisteredSources(String),
    /// A collection source could not be resolved or scanned.
    #[error(transparent)]
    FileSystem(#[from] FileSystemError),
    /// The system clock could not provide collection creation metadata.
    #[error(transparent)]
    Clock(#[from] ClockError),
}
