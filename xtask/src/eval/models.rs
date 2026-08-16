#![forbid(unsafe_code)]

//! Data models for the search quality evaluation framework.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single markdown document in the evaluation corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorpusDoc {
    /// Unique document identifier.
    pub doc_id: String,
    /// Path of the file relative to the collection root.
    pub path: String,
    /// Title of the document.
    pub title: String,
    /// Frontmatter metadata map.
    #[serde(default)]
    pub frontmatter: HashMap<String, serde_json::Value>,
    /// Full raw Markdown text content.
    pub content: String,
}

/// A search query entry in the evaluation query set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryItem {
    /// Unique query identifier.
    pub query_id: String,
    /// The query text string.
    pub query_text: String,
    /// Modality category of the query (e.g. lexical, semantic, contextual, `hard_negative`).
    pub modality: String,
    /// Descriptive intent or explanation of the query.
    #[serde(default)]
    pub description: String,
}

/// A ground truth relevance judgment mapping a query to a target passage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QrelItem {
    /// The query identifier being evaluated.
    pub query_id: String,
    /// The document identifier containing the relevant content.
    pub doc_id: String,
    /// 1-indexed start line in the document.
    pub line_start: usize,
    /// 1-indexed end line in the document.
    pub line_end: usize,
    /// Graded relevance score: 0 (irrelevant), 1 (relevant context), 2 (exact target passage).
    pub score: u8,
}

/// A single retrieved item for a query at an assigned rank.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedItem {
    /// The document identifier retrieved.
    pub doc_id: String,
    /// The start line of the passage.
    pub line_start: usize,
    /// The end line of the passage.
    pub line_end: usize,
    /// Score or similarity assigned by the search engine.
    pub score: f64,
}

/// Metrics summary for a specific query modality or aggregate set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModalityMetrics {
    /// Modality name (e.g. "all", "lexical", "semantic", "contextual", "`hard_negative`").
    pub modality: String,
    /// Total number of queries in this category.
    pub query_count: usize,
    /// Hit rate (Recall@K): proportion of queries with at least one relevant passage in top-K.
    pub recall_at_k: f64,
    /// Mean Reciprocal Rank (MRR@K).
    pub mrr_at_k: f64,
    /// Normalized Discounted Cumulative Gain (NDCG@K).
    pub ndcg_at_k: f64,
}

/// Complete evaluation report with overall and per-modality breakdowns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationReport {
    /// Value of K used for top-K evaluation.
    pub cutoff_k: usize,
    /// Total queries evaluated.
    pub total_queries: usize,
    /// Overall metrics aggregated across all queries.
    pub overall: ModalityMetrics,
    /// Per-modality breakdown.
    pub modalities: Vec<ModalityMetrics>,
}
