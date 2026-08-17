#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure domain types and rules for `mdsearch`.

mod collection;
mod content_hash;
mod passage;
mod timestamp;

pub use collection::{CollectionName, CollectionNameError};
pub use content_hash::{ContentHash, ContentHashError};
pub use passage::{FrontmatterIssue, Passage, PassageKind, segment_passages};
pub use timestamp::Timestamp;
