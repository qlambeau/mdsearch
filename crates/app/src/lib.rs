#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Composition root and CLI application for `mdsearch`.

mod cli;
mod error;
mod run;

pub use error::AppError;
pub use run::run;
