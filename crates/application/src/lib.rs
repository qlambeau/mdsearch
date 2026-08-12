#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `kv`.

mod create_collection;
mod error;
mod ports;

pub use create_collection::CreateCollection;
pub use error::{ClockError, CollectionStoreError, CreateCollectionError};
pub use ports::{Clock, CollectionStore};
