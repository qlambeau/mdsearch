//! Acceptance tests for the `mdsearch get` command.

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

fn add_file(
    home: &Path,
    collection: &str,
    file: &Path,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
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
    Ok(())
}

fn store_file(
    home: &Path,
    collection: &str,
    file: &Path,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    create(home, collection)?;
    add_file(home, collection, file, content)
}

/// Covers: FR-004 and FR-008 — retrieval by exact path prints the raw content.
#[test]
fn get_retrieves_by_exact_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("vault").join("notes.md");
    store_file(home.path(), "Notes", &file, "alpha")?;

    let output = run(
        ["mdsearch", "get", "Notes", path_argument(&file)?],
        home.path(),
    )?;

    assert_eq!(output, "alpha");

    Ok(())
}

/// Covers: FR-004 — retrieval by a unique basename.
#[test]
fn get_retrieves_by_unique_basename() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("vault").join("notes.md");
    store_file(home.path(), "Notes", &file, "alpha")?;

    let output = run(["mdsearch", "get", "Notes", "notes.md"], home.path())?;

    assert_eq!(output, "alpha");

    Ok(())
}

/// Covers: FR-003 — retrieval by an indexing-assigned ID.
#[test]
fn get_retrieves_by_id() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let first = home.path().join("a.md");
    let second = home.path().join("b.md");
    store_file(home.path(), "Notes", &first, "alpha")?;
    fs::write(&second, "beta")?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&second)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "get", "Notes", "2"], home.path())?;

    assert_eq!(output, "beta");

    Ok(())
}

/// Covers: FR-005 — an ambiguous basename lists the candidate paths.
#[test]
fn get_reports_an_ambiguous_basename_with_candidates() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a").join("x.md");
    let b = home.path().join("b").join("x.md");
    create(home.path(), "Notes")?;
    add_file(home.path(), "Notes", &a, "one")?;
    add_file(home.path(), "Notes", &b, "two")?;

    let error = run(["mdsearch", "get", "Notes", "x.md"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an ambiguous basename should fail"))?;

    let message = error.to_string();
    assert!(message.contains("ambiguous"), "unexpected error: {message}");
    assert!(message.contains("x.md"));

    Ok(())
}

/// Covers: FR-006 — a missing name reports not found.
#[test]
fn get_reports_a_file_not_found_by_name() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    store_file(home.path(), "Notes", &file, "alpha")?;

    let error = run(["mdsearch", "get", "Notes", "missing.md"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("a missing file should fail"))?;

    assert!(
        error.to_string().contains("not found"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: FR-006 — a missing ID reports not found.
#[test]
fn get_reports_a_file_not_found_by_id() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    store_file(home.path(), "Notes", &file, "alpha")?;

    let error = run(["mdsearch", "get", "Notes", "999"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("a missing file should fail"))?;

    assert!(
        error.to_string().contains("not found"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: FR-002 — a missing collection reports not found.
#[test]
fn get_reports_a_missing_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    store_file(home.path(), "Notes", &file, "alpha")?;

    let error = run(["mdsearch", "get", "Journal", "notes.md"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("a missing collection should fail"))?;

    assert!(
        error.to_string().contains("not found"),
        "unexpected error: {error}"
    );

    Ok(())
}

/// Covers: FR-007 — a missing database fails without creating a file.
#[test]
fn get_reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "get",
            "Notes",
            "notes.md",
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
