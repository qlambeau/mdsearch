#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Composition root and CLI application for `mdsearch`.

mod cli;
mod error;
mod graph_query;
mod run;

pub use error::AppError;
pub use graph_query::{GraphQueryRoot, build_schema, handle};
pub use run::run;
