//! Acceptance tests for the lexical-search application use case.

use std::error::Error;
use std::path::PathBuf;

use kv_application::{
    LexicalSearchStore, SearchError, SearchLexical, SearchResult, SearchResultSet, SearchScope,
    SearchStoreError,
};
use kv_domain::{CollectionName, PassageKind};

#[derive(Default)]
struct InMemorySearchStore {
    result_set: Option<SearchResultSet>,
    fail_not_found: bool,
}

impl InMemorySearchStore {
    fn set_results(&mut self, set: SearchResultSet) {
        self.result_set = Some(set);
    }
}

impl LexicalSearchStore for InMemorySearchStore {
    fn search(
        &self,
        _query: &str,
        _limit: usize,
        _scope: SearchScope<'_>,
    ) -> Result<SearchResultSet, SearchStoreError> {
        if self.fail_not_found {
            return Err(SearchStoreError::CollectionNotFound);
        }
        self.result_set
            .clone()
            .ok_or_else(|| SearchStoreError::Storage(Box::new(std::io::Error::other("no results"))))
    }
}

fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
    CollectionName::try_from(name)
}

fn result(
    path: &str,
    kind: PassageKind,
    text: &str,
    score: f64,
) -> Result<SearchResult, Box<dyn Error>> {
    Ok(SearchResult::new(
        collection("Notes")?,
        PathBuf::from(path),
        kind,
        text.to_owned(),
        score,
    ))
}

/// Covers: FR-001 and FR-004 — results are returned in ranked order.
#[test]
fn returns_ranked_results_for_the_all_scope() -> Result<(), Box<dyn Error>> {
    let mut store = InMemorySearchStore::default();
    store.set_results(SearchResultSet::new(
        vec![
            result("/a.md", PassageKind::Body, "borrowing rules", 6.0)?,
            result("/b.md", PassageKind::Body, "borrowing quirks", 3.0)?,
        ],
        2,
    ));
    let use_case = SearchLexical::new(store);

    let set = use_case.execute("borrowing", 10, SearchScope::All)?;

    assert_eq!(set.total(), 2);
    let results = set.results();
    assert_eq!(results.len(), 2);
    assert_eq!(
        results.first().map(SearchResult::text),
        Some("borrowing rules")
    );
    assert_eq!(
        results.get(1).map(SearchResult::text),
        Some("borrowing quirks")
    );

    Ok(())
}

/// Covers: FR-002 — the collection scope is forwarded to the store.
#[test]
fn forwards_a_collection_scope() -> Result<(), Box<dyn Error>> {
    let mut store = InMemorySearchStore::default();
    store.set_results(SearchResultSet::new(
        vec![result("/a.md", PassageKind::Body, "borrowing", 1.0)?],
        1,
    ));
    let use_case = SearchLexical::new(store);

    let set = use_case.execute(
        "borrowing",
        10,
        SearchScope::Collection(&collection("Notes")?),
    )?;

    assert_eq!(set.total(), 1);

    Ok(())
}

/// Covers: FR-012 — no matches produce an empty result set.
#[test]
fn returns_an_empty_result_set_for_no_matches() -> Result<(), Box<dyn Error>> {
    let mut store = InMemorySearchStore::default();
    store.set_results(SearchResultSet::new(Vec::new(), 0));
    let use_case = SearchLexical::new(store);

    let set = use_case.execute("zzznotaword", 10, SearchScope::All)?;

    assert!(set.results().is_empty());
    assert_eq!(set.total(), 0);

    Ok(())
}

/// Covers: FR-006 — an empty query is rejected before reaching the store.
#[test]
fn rejects_an_empty_query() {
    let store = InMemorySearchStore::default();
    let use_case = SearchLexical::new(store);

    assert!(matches!(
        use_case.execute("   ", 10, SearchScope::All),
        Err(SearchError::EmptyQuery)
    ));
}

/// Covers: FR-008 — store errors propagate.
#[test]
fn propagates_store_errors() -> Result<(), Box<dyn Error>> {
    let store = InMemorySearchStore {
        fail_not_found: true,
        ..InMemorySearchStore::default()
    };
    let use_case = SearchLexical::new(store);
    let notes = CollectionName::try_from("Notes")?;

    assert!(matches!(
        use_case.execute("borrowing", 10, SearchScope::Collection(&notes)),
        Err(SearchError::Store(SearchStoreError::CollectionNotFound))
    ));

    Ok(())
}
