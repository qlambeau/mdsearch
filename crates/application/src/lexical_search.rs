use kv_domain::free_text_to_fts5;

use crate::{LexicalSearchStore, SearchError, SearchResultSet, SearchScope};

/// Searches the built lexical index for ranked passages.
pub struct SearchLexical<S> {
    store: S,
}

impl<S> SearchLexical<S>
where
    S: LexicalSearchStore,
{
    /// Creates a lexical-search use case with its store port.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns ranked passages matching `query` within `scope`.
    ///
    /// The query is literal free text: it is mapped to a quoted, AND-joined
    /// FTS5 expression so operator characters match literally, identically to
    /// the hybrid command.
    ///
    /// # Errors
    ///
    /// Returns an empty-query error when the query is empty or whitespace-only,
    /// or a store error when the search cannot complete.
    pub fn execute(
        &self,
        query: &str,
        limit: usize,
        scope: SearchScope<'_>,
    ) -> Result<SearchResultSet, SearchError> {
        if query.trim().is_empty() {
            return Err(SearchError::EmptyQuery);
        }

        let fts5_query = free_text_to_fts5(query).ok_or(SearchError::EmptyQuery)?;

        Ok(self.store.search(&fts5_query, limit, scope)?)
    }
}
