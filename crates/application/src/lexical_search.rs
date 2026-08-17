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

        Ok(self.store.search(query, limit, scope)?)
    }
}
