#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure domain types and rules for `mdsearch`.

mod collection;
mod content_hash;
mod embedding;
mod file_id;
mod passage;
mod timestamp;

pub use collection::{CollectionName, CollectionNameError};
pub use content_hash::{ContentHash, ContentHashError};
pub use embedding::{
    Embedding, EmbeddingModel, EmbeddingModelError, SemanticIndexStatus, SemanticPassage,
    file_set_fingerprint,
};
pub use file_id::{FileId, FileIdError};
pub use passage::{FrontmatterIssue, Passage, PassageKind, segment_passages};
pub use timestamp::Timestamp;
