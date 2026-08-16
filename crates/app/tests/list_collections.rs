//! Acceptance tests for the `mdsearch collection list` command.

use std::error::Error;
use std::process::Command;

use tempfile::tempdir;

use kv_app::run;
use kv_store_sqlite::SqliteCollectionStore;

/// Covers: FR-001, FR-002, and FR-004 — list the default database.
#[test]
fn lists_collections_at_the_default_database_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(["mdsearch", "collection", "create", "Archive"], home.path())?;

    let output = run(["mdsearch", "collection", "list"], home.path())?;

    assert_eq!(output, "Archive\nNotes");

    Ok(())
}

/// Covers: FR-004 — names are listed case-insensitively sorted.
#[test]
fn lists_collections_in_case_insensitive_alphabetical_order() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    for name in ["banana", "Apple", "cherry"] {
        run(["mdsearch", "collection", "create", name], home.path())?;
    }

    let output = run(["mdsearch", "collection", "list"], home.path())?;

    assert_eq!(output, "Apple\nbanana\ncherry");

    Ok(())
}

/// Covers: FR-005 — an existing empty database produces no output.
#[test]
fn lists_nothing_for_an_existing_empty_database() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join(".mdsearch").join("collections.db");
    SqliteCollectionStore::open(&database_path)?;

    let output = run(["mdsearch", "collection", "list"], home.path())?;

    assert_eq!(output, "");

    Ok(())
}

/// Covers: FR-003 and FR-006 — an explicit missing database fails without creation.
#[test]
fn fails_for_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "list",
            "--database",
            database_argument,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing database should fail to list"))?;

    assert!(error.to_string().contains("does not exist"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-007 — an unopenable database fails semantically.
#[test]
fn fails_for_an_unopenable_database() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("a_directory_as_database");
    std::fs::create_dir(&database_path)?;
    let database_argument = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "list",
            "--database",
            database_argument,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("an unopenable database should fail to list"))?;

    assert!(error.to_string().contains("database"));

    Ok(())
}

/// Covers: FR-003 — the explicit database path is listed.
#[test]
fn lists_collections_at_the_explicit_database_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))?;

    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Project Notes",
            "--database",
            database_argument,
        ],
        home.path(),
    )?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "list",
            "--database",
            database_argument,
        ],
        home.path(),
    )?;

    assert_eq!(output, "Project Notes");

    Ok(())
}

/// Covers: FR-009 — a collection created in an earlier run remains listed.
#[test]
fn lists_a_collection_across_cli_runs() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(["mdsearch", "collection", "list"], home.path())?;

    assert_eq!(output, "Notes");

    Ok(())
}

/// Covers: the single-binary success path for listing.
#[test]
fn binary_lists_collections() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "list"])
        .env("HOME", home.path())
        .output()?;

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "Notes\n");

    Ok(())
}

/// Covers: FR-005 — the binary emits no bytes for an empty database.
#[test]
fn binary_emits_nothing_for_an_empty_database() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join(".mdsearch").join("collections.db");
    SqliteCollectionStore::open(&database_path)?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "list"])
        .env("HOME", home.path())
        .output()?;

    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    Ok(())
}
