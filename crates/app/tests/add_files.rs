//! Acceptance tests for the `mdsearch collection add` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

/// Covers: FR-007, FR-008, and FR-013 — a directory is added recursively.
#[test]
fn adds_files_from_a_directory_recursively() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(vault.join("sub"))?;
    fs::write(vault.join("a.md"), "alpha")?;
    fs::write(vault.join("sub").join("b.md"), "beta")?;
    fs::write(vault.join("readme.txt"), "not markdown")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    assert_eq!(output, "added 2 files to collection \"Notes\"");

    Ok(())
}

/// Covers: FR-001 and FR-013 — a single file is added.
#[test]
fn adds_a_single_file() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    assert_eq!(output, "added 1 file to collection \"Notes\"");

    Ok(())
}

/// Covers: FR-009 — re-adding reports one file (no duplicate).
#[test]
fn re_adding_a_file_reports_one_file() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
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

    let output = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    assert_eq!(output, "added 1 file to collection \"Notes\"");

    Ok(())
}

/// Covers: FR-011 — an unreadable path fails.
#[test]
fn fails_when_a_path_is_unreadable() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    let missing = home.path().join("missing.md");

    let error = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&missing)?,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unreadable path should fail"))?;

    assert!(error.to_string().contains("unreadable"));

    Ok(())
}

/// Covers: FR-012 — `--force` skips unreadable paths and reports the skip.
#[test]
fn skips_unreadable_paths_with_force() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;
    let missing = home.path().join("missing.md");

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&missing)?,
            path_argument(&file)?,
            "--force",
        ],
        home.path(),
    )?;

    assert_eq!(output, "added 1 file to collection \"Notes\" (skipped 1)");

    Ok(())
}

/// Covers: FR-005 — a missing collection fails.
#[test]
fn reports_a_missing_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;
    run(["mdsearch", "collection", "create", "Other"], home.path())?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing collection should fail"))?;

    assert!(error.to_string().contains("not found"));

    Ok(())
}

/// Covers: FR-006 — a missing database fails without creation.
#[test]
fn reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
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

/// Covers: FR-003 — the explicit database path is used.
#[test]
fn adds_files_to_an_explicit_database() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("notes.md");
    fs::write(&file, "content")?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            "--database",
            database_argument,
        ],
        home.path(),
    )?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&file)?,
            "--database",
            database_argument,
        ],
        home.path(),
    )?;

    assert_eq!(output, "added 1 file to collection \"Notes\"");

    Ok(())
}
