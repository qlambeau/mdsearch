//! Acceptance tests for the `mdsearch collection destroy` command.

use std::error::Error;
use std::process::Command;

use rstest::rstest;
use tempfile::tempdir;

use kv_app::run;

/// Covers: FR-001, FR-002, FR-008, and FR-010 — destroy at the default database.
#[test]
fn destroys_a_collection_at_the_default_database_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(["mdsearch", "collection", "destroy", "Notes"], home.path())?;

    assert_eq!(output, "destroyed collection \"Notes\"");
    assert_eq!(run(["mdsearch", "collection", "list"], home.path())?, "");

    Ok(())
}

/// Covers: FR-004 and FR-008 — matching is case-insensitive and reports retained spelling.
#[test]
fn destroys_a_collection_case_insensitively() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(["mdsearch", "collection", "destroy", "notes"], home.path())?;

    assert_eq!(output, "destroyed collection \"Notes\"");

    Ok(())
}

/// Covers: FR-009 — destroying one collection leaves others intact.
#[test]
fn destroys_one_collection_without_disturbing_others() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(["mdsearch", "collection", "create", "Archive"], home.path())?;

    run(["mdsearch", "collection", "destroy", "Notes"], home.path())?;

    assert_eq!(
        run(["mdsearch", "collection", "list"], home.path())?,
        "Archive"
    );

    Ok(())
}

/// Covers: FR-006 — destroying a non-existent collection fails without change.
#[test]
fn fails_for_a_non_existent_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let error = run(
        ["mdsearch", "collection", "destroy", "Missing"],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing collection should fail to destroy"))?;

    assert!(error.to_string().contains("not found"));
    assert_eq!(
        run(["mdsearch", "collection", "list"], home.path())?,
        "Notes"
    );

    Ok(())
}

/// Covers: FR-007 — destroying in a missing database fails without creation.
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
            "destroy",
            "Notes",
            "--database",
            database_argument,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("a missing database should fail to destroy"))?;

    assert!(error.to_string().contains("does not exist"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: FR-005 — invalid names are rejected without destroying anything.
#[rstest]
#[case("")]
#[case("   ")]
#[case("Notes/2026")]
#[case("Notes\\2026")]
#[case("Notes\n2026")]
fn rejects_invalid_collection_names(#[case] name: &str) -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let error = run(["mdsearch", "collection", "destroy", name], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an invalid collection name should fail"))?;

    assert!(error.to_string().contains("collection name"));
    assert_eq!(
        run(["mdsearch", "collection", "list"], home.path())?,
        "Notes"
    );

    Ok(())
}

/// Covers: FR-003 — the explicit database path is used.
#[test]
fn destroys_a_collection_at_the_explicit_database_path() -> Result<(), Box<dyn Error>> {
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
            "destroy",
            "Project Notes",
            "--database",
            database_argument,
        ],
        home.path(),
    )?;

    assert_eq!(output, "destroyed collection \"Project Notes\"");

    Ok(())
}

/// Covers: the single-binary success path for destroy.
#[test]
fn binary_destroys_a_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "destroy", "Notes"])
        .env("HOME", home.path())
        .output()?;

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "destroyed collection \"Notes\"\n"
    );

    Ok(())
}
