#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Composition root and CLI application for `mdsearch`.

mod cli;
mod error;
mod graph_query;
mod model_assets;
mod model_cache;
mod models;
mod output;
mod progress;
mod recovery;
mod related;
mod rendering;
mod run;
mod status_rendering;

pub use error::AppError;
pub use graph_query::{GraphQueryRoot, build_schema};
pub use output::CommandOutput;
pub use related::{RelatedFile, related_files};
pub use run::{run, run_from_environment};
