#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure domain types and rules for `mdsearch`.

mod collection;
mod content_hash;
mod embedding;
mod file_id;
mod fusion;
mod passage;
mod reranking;
mod timestamp;

pub use collection::{CollectionName, CollectionNameError};
pub use content_hash::{ContentHash, ContentHashError};
pub use embedding::{
    Embedding, EmbeddingModel, EmbeddingModelError, SemanticIndexStatus, SemanticPassage,
    file_set_fingerprint,
};
pub use file_id::{FileId, FileIdError};
pub use fusion::{DEFAULT_RRF_K, FusedRank, PassageKey, free_text_to_fts5, reciprocal_rank_fusion};
pub use passage::{FrontmatterIssue, Passage, PassageKind, segment_passages};
pub use reranking::{RerankerModel, RerankerModelError};
pub use timestamp::Timestamp;
