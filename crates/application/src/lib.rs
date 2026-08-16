#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `mdsearch`.

mod create_collection;
mod error;
mod list_collections;
mod ports;

pub use create_collection::CreateCollection;
pub use error::{ClockError, CollectionStoreError, CreateCollectionError, ListCollectionsError};
pub use list_collections::ListCollections;
pub use ports::{Clock, CollectionStore};
