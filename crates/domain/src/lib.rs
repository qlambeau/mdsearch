#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure domain types and rules for `mdsearch`.

mod collection;
mod content_hash;
mod timestamp;

pub use collection::{CollectionName, CollectionNameError};
pub use content_hash::{ContentHash, ContentHashError};
pub use timestamp::Timestamp;
