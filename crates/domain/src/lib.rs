#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure domain types and rules for `kv`.

mod collection;
mod timestamp;

pub use collection::{CollectionName, CollectionNameError};
pub use timestamp::Timestamp;
