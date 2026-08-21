use std::path::{Path, PathBuf};

use kv_domain::{CollectionName, PassageKind};

use crate::SearchStoreError;

/// The scope of a lexical search.
#[derive(Clone, Copy, Debug)]
pub enum SearchScope<'a> {
    /// Search every collection with a built index.
    All,
    /// Search one named collection.
    Collection(&'a CollectionName),
}

/// The location of a passage within its file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    byte_offset: usize,
    byte_length: usize,
    line_start: usize,
    line_end: usize,
}

impl Position {
    /// Creates a position from its byte and 1-based inclusive line ranges.
    #[must_use]
    pub const fn new(
        byte_offset: usize,
        byte_length: usize,
        line_start: usize,
        line_end: usize,
    ) -> Self {
        Self {
            byte_offset,
            byte_length,
            line_start,
            line_end,
        }
    }

    /// Returns the byte offset of the passage in its file.
    #[must_use]
    pub const fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Returns the byte length of the passage.
    #[must_use]
    pub const fn byte_length(&self) -> usize {
        self.byte_length
    }

    /// Returns the 1-based inclusive first line of the passage.
    #[must_use]
    pub const fn line_start(&self) -> usize {
        self.line_start
    }

    /// Returns the 1-based inclusive last line of the passage.
    #[must_use]
    pub const fn line_end(&self) -> usize {
        self.line_end
    }
}

/// One ranked passage match.
#[derive(Clone, Debug)]
pub struct SearchResult {
    collection: CollectionName,
    path: PathBuf,
    kind: PassageKind,
    text: String,
    score: f64,
    position: Position,
}

impl SearchResult {
    /// Creates a search result record.
    #[must_use]
    pub fn new(
        collection: CollectionName,
        path: PathBuf,
        kind: PassageKind,
        text: String,
        score: f64,
        position: Position,
    ) -> Self {
        Self {
            collection,
            path,
            kind,
            text,
            score,
            position,
        }
    }

    /// Returns the collection the passage belongs to.
    #[must_use]
    pub fn collection(&self) -> &CollectionName {
        &self.collection
    }

    /// Returns the file path of the passage.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the passage kind.
    #[must_use]
    pub const fn kind(&self) -> PassageKind {
        self.kind
    }

    /// Returns the passage text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the BM25 score (higher is better).
    #[must_use]
    pub const fn score(&self) -> f64 {
        self.score
    }

    /// Returns the passage's position in its file.
    #[must_use]
    pub const fn position(&self) -> Position {
        self.position
    }
}

/// The ranked results of a lexical search and the total match count.
#[derive(Clone, Debug)]
pub struct SearchResultSet {
    results: Vec<SearchResult>,
    total: usize,
}

impl SearchResultSet {
    /// Creates a result set with its ranked results and total count.
    #[must_use]
    pub const fn new(results: Vec<SearchResult>, total: usize) -> Self {
        Self { results, total }
    }

    /// Returns the ranked results, capped at the search limit.
    #[must_use]
    pub fn results(&self) -> &[SearchResult] {
        &self.results
    }

    /// Returns the total number of matching passages.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }
}

/// Searches the built lexical index.
pub trait LexicalSearchStore {
    /// Searches the lexical index within `scope`, returning up to `limit`
    /// ranked results ordered by BM25 score (highest first) and the total
    /// number of matching passages.
    ///
    /// `query` is a neutralized FTS5 expression produced by the domain
    /// free-text mapper (`free_text_to_fts5`): every term is a quoted phrase
    /// joined with `AND`, so it is valid FTS5 by construction. Callers must
    /// not pass raw user free text.
    ///
    /// For [`SearchScope::All`], collections without a built index are skipped.
    /// For [`SearchScope::Collection`], an unknown collection or a collection
    /// without a built index is an error.
    ///
    /// # Errors
    ///
    /// Returns a not-found, not-built, or storage error when the search cannot
    /// complete. The invalid-query variant is defense-in-depth and is not
    /// constructed from normal input (ADR-009).
    fn search(
        &self,
        query: &str,
        limit: usize,
        scope: SearchScope<'_>,
    ) -> Result<SearchResultSet, SearchStoreError>;
}
