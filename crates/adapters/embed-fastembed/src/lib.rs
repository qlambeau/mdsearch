#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `fastembed` adapter for the `mdsearch` embedding generator and re-ranker
//! ports.

mod embedding;
mod rerank;

pub use embedding::FastembedGenerator;
pub use rerank::FastembedReranker;
