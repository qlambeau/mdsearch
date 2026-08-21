//! Acceptance tests for the `mdsearch hybrid` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;
use kv_application::SemanticIndexStore;
use kv_domain::{CollectionName, Embedding, EmbeddingModel, SemanticPassage, Timestamp};
use kv_store_sqlite::SqliteSemanticIndexStore;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

fn create(home: &Path, collection: &str) -> Result<(), Box<dyn Error>> {
    run(["mdsearch", "collection", "create", collection], home)?;
    Ok(())
}

fn add_and_update(
    home: &Path,
    collection: &str,
    file: &Path,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    fs::write(file, content)?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            collection,
            path_argument(file)?,
        ],
        home,
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            collection,
            path_argument(file)?,
        ],
        home,
    )?;
    Ok(())
}

/// Builds a semantic index for "Notes" with fake vectors at `dimension`.
fn embed_with_dimension(home: &Path, dimension: usize) -> Result<(), Box<dyn Error>> {
    let database_path = home.join(".mdsearch").join("collections.db");
    let notes = CollectionName::try_from("Notes")?;
    let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    store.ensure_dimension(dimension)?;
    let passages = store.passages(&notes)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, Embedding::new(vec![0.1; dimension])))
        .collect::<Vec<(SemanticPassage, Embedding)>>();
    store.rebuild(
        &notes,
        &EmbeddingModel::try_new("all-MiniLM-L6-v2")?,
        Timestamp::from_unix_seconds(1_700_000_000),
        &pairs,
    )?;
    Ok(())
}

/// Covers: REQ-011 FR-003 — an empty query fails.
#[test]
fn hybrid_fails_on_an_empty_query() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let error = run(["mdsearch", "hybrid", ""], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an empty query should fail"))?;

    assert!(
        error.to_string().contains("query is empty"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: REQ-014 FR-002 — FTS5 operator characters match literally in the
/// hybrid lexical leg (offline-reachable without semantic assets).
#[test]
fn hybrid_matches_operator_characters_literally() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "a AND b semantics")?;
    add_and_update(home.path(), "Notes", &b, "borrowing only")?;

    let output = run(["mdsearch", "hybrid", "a AND"], home.path())?;

    assert!(
        output.contains("a AND b semantics"),
        "unexpected output: {output}"
    );
    assert!(
        !output.contains("borrowing only"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: REQ-014 FR-004 — the identical query string returns the same
/// passages on `search` and `hybrid` against the same collection state.
#[test]
fn search_and_hybrid_return_the_same_passages_for_the_same_query() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "a AND b semantics")?;
    add_and_update(home.path(), "Notes", &b, "borrowing only")?;

    let search = run(["mdsearch", "search", "a AND"], home.path())?;
    let hybrid = run(["mdsearch", "hybrid", "a AND"], home.path())?;

    for output in [&search, &hybrid] {
        assert!(
            output.contains("a AND b semantics"),
            "unexpected output: {output}"
        );
        assert!(
            !output.contains("borrowing only"),
            "unexpected output: {output}"
        );
    }

    Ok(())
}

/// Covers: REQ-011 FR-019 — a recorded dimension disagreeing with the active
/// dimension fails the hybrid command with a clear dimension-mismatch error.
#[test]
fn hybrid_reports_a_dimension_mismatch() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;
    embed_with_dimension(home.path(), 384)?;

    let database_path = home.path().join(".mdsearch").join("collections.db");
    let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    store.ensure_dimension(1024)?;
    drop(store);

    let error = run(["mdsearch", "hybrid", "borrowing"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("a dimension mismatch should fail"))?;

    assert!(
        error.to_string().contains("dimension mismatch"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: REQ-011 FR-013 — a missing database fails without creating a file.
#[test]
fn hybrid_reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "hybrid",
            "borrowing",
            "--database",
            database_argument,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(error.to_string().contains("does not exist"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: REQ-011 FR-009 — a targeted unknown collection fails.
#[test]
fn hybrid_reports_a_missing_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;

    let error = run(
        ["mdsearch", "hybrid", "borrowing", "--collection", "Journal"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unknown collection should fail"))?;

    assert!(error.to_string().contains("not found"));

    Ok(())
}

/// Covers: REQ-011 FR-009 — a targeted unbuilt lexical index fails.
#[test]
fn hybrid_reports_an_unbuilt_index() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    fs::write(&file, "borrowing")?;
    create(home.path(), "Notes")?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let error = run(
        ["mdsearch", "hybrid", "borrowing", "--collection", "Notes"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unbuilt index should fail"))?;

    assert!(error.to_string().contains("not built"));

    Ok(())
}

/// Covers: REQ-011 FR-003 — an out-of-range --limit fails.
#[test]
fn hybrid_rejects_an_out_of_range_limit() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let error = run(
        ["mdsearch", "hybrid", "borrowing", "--limit", "200"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an out-of-range limit should fail"))?;

    assert!(!error.to_string().is_empty());

    Ok(())
}

/// Covers: REQ-011 FR-007 — unbuilt collections are skipped when searching all.
#[test]
fn hybrid_skips_unbuilt_collections_when_searching_all() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let d = home.path().join("d.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;
    fs::write(&d, "borrowing")?;
    create(home.path(), "Draft")?;
    run(
        ["mdsearch", "collection", "add", "Draft", path_argument(&d)?],
        home.path(),
    )?;

    let output = run(["mdsearch", "hybrid", "borrowing"], home.path())?;

    assert!(output.contains("borrowing"));
    assert!(
        output.contains("1 result(s)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: REQ-011 FR-015 — no matches produce empty output.
#[test]
fn hybrid_produces_empty_output_when_nothing_matches() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;

    let output = run(["mdsearch", "hybrid", "zzznotaword"], home.path())?;

    assert_eq!(output, "");
    Ok(())
}

/// Covers: REQ-011 FR-018 — `--json` emits a structured object with results.
#[test]
fn hybrid_json_emits_a_structured_object() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;

    let output = run(["mdsearch", "hybrid", "borrowing", "--json"], home.path())?;

    let value: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(
        value.get("query").and_then(serde_json::Value::as_str),
        Some("borrowing")
    );
    assert_eq!(
        value.get("scope").and_then(serde_json::Value::as_str),
        Some("all")
    );
    assert_eq!(
        value.get("limit").and_then(serde_json::Value::as_u64),
        Some(10)
    );

    let first = first_result(&value)?;
    assert_result_provenance(first);
    assert_result_scores(first, &output);

    Ok(())
}

fn assert_result_provenance(result: &serde_json::Value) {
    assert_eq!(
        result.get("collection").and_then(serde_json::Value::as_str),
        Some("Notes")
    );
    assert_eq!(
        result.get("kind").and_then(serde_json::Value::as_str),
        Some("body")
    );
    assert_eq!(
        result.get("text").and_then(serde_json::Value::as_str),
        Some("borrowing rules")
    );
}

fn assert_result_scores(result: &serde_json::Value, output: &str) {
    assert!(has_f64_field(result, "fused_score"));
    assert!(has_f64_field(result, "bm25_score"));
    assert!(has_f64_field(result, "ordering_score"));
    assert!(
        has_field(result, "reranker_score"),
        "unexpected output: {output}"
    );
    assert!(
        has_field(result, "cosine_similarity"),
        "unexpected output: {output}"
    );
}

fn first_result(value: &serde_json::Value) -> Result<&serde_json::Value, std::io::Error> {
    let results = value
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("results must be an array"))?;
    results
        .first()
        .ok_or_else(|| std::io::Error::other("expected a result"))
}

fn has_f64_field(value: &serde_json::Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(serde_json::Value::as_f64)
        .is_some()
}

fn has_field(value: &serde_json::Value, field: &str) -> bool {
    value.get(field).is_some()
}

/// Covers: REQ-011 FR-012 — a human run caps blocks and reports the shown count.
#[test]
fn hybrid_human_reports_the_shown_count() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    create(home.path(), "Notes")?;
    for name in ["a.md", "b.md", "c.md"] {
        let file = home.path().join(name);
        add_and_update(home.path(), "Notes", &file, "borrowing")?;
    }

    let output = run(
        [
            "mdsearch",
            "hybrid",
            "borrowing",
            "--no-rerank",
            "--limit",
            "2",
        ],
        home.path(),
    )?;

    let block_count = output
        .lines()
        .filter(|line| line.contains("(body, score "))
        .count();
    assert_eq!(block_count, 2, "unexpected output: {output}");
    assert!(
        output.contains("2 result(s)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: REQ-011 FR-006 — an uncached re-ranker falls back with a warning.
#[test]
fn hybrid_warns_when_the_reranker_is_uncached() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;

    let output = run(["mdsearch", "hybrid", "borrowing"], home.path())?;

    assert!(output.contains("borrowing rules"));
    assert!(
        output.contains("re-ranking skipped"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: REQ-011 FR-006 — `--no-rerank` suppresses the warning.
#[test]
fn hybrid_no_rerank_suppresses_the_warning() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;

    let output = run(
        ["mdsearch", "hybrid", "borrowing", "--no-rerank"],
        home.path(),
    )?;

    assert!(output.contains("borrowing rules"));
    assert!(
        !output.contains("re-ranking skipped"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: REQ-011 FR-012 — `--limit` caps results and JSON reports the count.
#[test]
fn hybrid_json_honors_limit() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;

    let output = run(
        ["mdsearch", "hybrid", "borrowing", "--json", "--limit", "1"],
        home.path(),
    )?;

    let value: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(
        value.get("limit").and_then(serde_json::Value::as_u64),
        Some(1)
    );
    let results = value
        .get("results")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("results must be an array"))?;
    assert_eq!(results.len(), 1);

    Ok(())
}
