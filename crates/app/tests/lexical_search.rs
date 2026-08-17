//! Acceptance tests for the `mdsearch search` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;

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

/// Covers: FR-001 and FR-010 — search returns ranked passage blocks and a summary.
#[test]
fn search_returns_ranked_passage_blocks_and_a_summary() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    create(home.path(), "Notes")?;
    create(home.path(), "Archive")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;
    add_and_update(home.path(), "Archive", &b, "borrowing anywhere")?;

    let output = run(["mdsearch", "search", "borrowing"], home.path())?;

    assert!(
        output.contains(".md (body, score "),
        "unexpected output: {output}"
    );
    assert!(output.contains("borrowing rules"));
    assert!(output.contains("borrowing anywhere"));
    assert!(
        output.contains("2 match(es)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: FR-002 — --collection restricts the search.
#[test]
fn search_restricts_to_a_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    create(home.path(), "Notes")?;
    create(home.path(), "Archive")?;
    add_and_update(home.path(), "Notes", &a, "borrowing rules")?;
    add_and_update(home.path(), "Archive", &b, "borrowing anywhere")?;

    let output = run(
        ["mdsearch", "search", "borrowing", "--collection", "Notes"],
        home.path(),
    )?;

    assert!(output.contains("borrowing rules"));
    assert!(!output.contains("borrowing anywhere"));
    assert!(
        output.contains("1 match(es)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: FR-008 — --collection on an unknown collection fails.
#[test]
fn search_reports_a_missing_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;

    let error = run(
        ["mdsearch", "search", "borrowing", "--collection", "Journal"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unknown collection should fail"))?;

    assert!(error.to_string().contains("not found"));

    Ok(())
}

/// Covers: FR-009 — --collection on an unbuilt index fails.
#[test]
fn search_reports_an_unbuilt_index() -> Result<(), Box<dyn Error>> {
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
        ["mdsearch", "search", "borrowing", "--collection", "Notes"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unbuilt index should fail"))?;

    assert!(error.to_string().contains("not built"));

    Ok(())
}

/// Covers: FR-007 — unbuilt collections are skipped when searching all.
#[test]
fn search_skips_unbuilt_collections_when_searching_all() -> Result<(), Box<dyn Error>> {
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

    let output = run(["mdsearch", "search", "borrowing"], home.path())?;

    assert!(
        output.contains("1 match(es)"),
        "unexpected output: {output}"
    );
    assert!(output.contains("borrowing"));

    Ok(())
}

/// Covers: FR-003 and FR-011 — --limit caps blocks and the summary reports the total.
#[test]
fn search_limits_results_and_reports_the_total() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    create(home.path(), "Notes")?;
    for name in ["a.md", "b.md", "c.md"] {
        let file = home.path().join(name);
        add_and_update(home.path(), "Notes", &file, "borrowing")?;
    }

    let output = run(
        ["mdsearch", "search", "borrowing", "--limit", "2"],
        home.path(),
    )?;

    let block_count = output
        .lines()
        .filter(|line| line.contains("(body, score "))
        .count();
    assert_eq!(block_count, 2, "unexpected output: {output}");
    assert!(
        output.contains("3 match(es)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: FR-003 — an out-of-range --limit fails.
#[test]
fn search_rejects_an_out_of_range_limit() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let error = run(
        ["mdsearch", "search", "borrowing", "--limit", "200"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an out-of-range limit should fail"))?;

    assert!(!error.to_string().is_empty());

    Ok(())
}

/// Covers: FR-006 — a phrase query matches only the phrase.
#[test]
fn search_matches_an_exact_phrase() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "rust ownership\n\nrust borrow")?;

    let output = run(["mdsearch", "search", "\"rust ownership\""], home.path())?;

    assert!(output.contains("rust ownership"));
    assert!(!output.contains("rust borrow"));
    assert!(
        output.contains("1 match(es)"),
        "unexpected output: {output}"
    );

    Ok(())
}

/// Covers: FR-006 — a malformed query fails with a clear error.
#[test]
fn search_fails_clearly_on_a_malformed_query() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;

    let error = run(["mdsearch", "search", "a AND"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("a malformed query should fail"))?;

    assert!(
        error.to_string().contains("invalid query"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: FR-006 — an empty query fails.
#[test]
fn search_fails_on_an_empty_query() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let error = run(["mdsearch", "search", ""], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an empty query should fail"))?;

    assert!(
        error.to_string().contains("query is empty"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: FR-012 — no matches produce empty output.
#[test]
fn search_produces_empty_output_when_nothing_matches() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    create(home.path(), "Notes")?;
    add_and_update(home.path(), "Notes", &a, "borrowing")?;

    let output = run(["mdsearch", "search", "zzznotaword"], home.path())?;

    assert_eq!(output, "");

    Ok(())
}

/// Covers: FR-013 — a missing database fails without creating a file.
#[test]
fn search_reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "search",
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
