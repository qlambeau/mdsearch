#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application use cases and ports for `mdsearch`.

mod create_collection;
mod destroy_collection;
mod error;
mod list_collections;
mod ports;

pub use create_collection::CreateCollection;
pub use destroy_collection::DestroyCollection;
pub use error::{
    ClockError, CollectionStoreError, CreateCollectionError, DestroyCollectionError,
    ListCollectionsError,
};
pub use list_collections::ListCollections;
pub use ports::{Clock, CollectionStore};
