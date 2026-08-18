//! Acceptance tests for the `mdsearch embed` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

/// Covers: REQ-017 — a missing database fails without creating a file.
#[test]
fn embed_missing_database_fails_without_creating_a_file() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("missing").join("collections.db");

    let error = run(
        [
            "mdsearch",
            "embed",
            "--database",
            path_argument(&database_path)?,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing database should fail"))?;

    assert!(error.to_string().contains("database does not exist"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: REQ-008 — an unsupported model fails before any collection work.
#[test]
fn embed_unsupported_model_fails_before_any_collection_work() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "borrowing rules")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    let error = run(["mdsearch", "embed", "--model", "bogus-model"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an unsupported model should fail"))?;

    assert!(error.to_string().contains("bogus-model"));

    Ok(())
}

/// Covers: REQ-010 — the CLI surfaces the download suggestion for an uncached model.
#[test]
fn embed_uncached_model_suggests_download() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "borrowing rules")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    let error = run(["mdsearch", "embed"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an uncached model should fail"))?;

    let message = error.to_string();
    assert!(message.contains("--download"));

    Ok(())
}

/// Covers: REQ-021 — an unsupported reranker fails before any collection work.
#[test]
fn embed_unsupported_reranker_fails() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "borrowing rules")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    let error = run(
        ["mdsearch", "embed", "--reranker", "bogus-reranker"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unsupported reranker should fail"))?;

    assert!(error.to_string().contains("bogus-reranker"));

    Ok(())
}

/// Covers: REQ-021 — an uncached reranker suggests download.
#[test]
fn embed_uncached_reranker_suggests_download() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "borrowing rules")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    let error = run(
        ["mdsearch", "embed", "--reranker", "bge-reranker-base"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an uncached reranker should fail"))?;

    assert!(error.to_string().contains("--download"));

    Ok(())
}
