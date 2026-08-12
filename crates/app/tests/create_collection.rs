//! Acceptance tests for the `mdsearch collection create` command.

use std::error::Error;
use std::process::Command;

use rstest::rstest;
use tempfile::tempdir;

use kv_app::run;

/// Covers: FR-001, FR-002, FR-007, and FR-008 — create the first collection.
#[test]
fn creates_a_collection_at_the_default_database_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let output = run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    assert!(output.contains("Notes"));
    assert!(home.path().join(".mdsearch/collections.db").exists());

    Ok(())
}

/// Covers: FR-003 and FR-007 — the explicit database path is used.
#[test]
fn creates_a_collection_at_the_explicit_database_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))?;

    let output = run(
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

    assert!(output.contains("Project Notes"));
    assert!(database_path.exists());
    assert!(!home.path().join(".mdsearch/collections.db").exists());

    Ok(())
}

/// Covers: FR-006, FR-009, and FR-010 — persisted names are case-insensitively unique.
#[test]
fn rejects_a_case_insensitive_duplicate_across_cli_runs() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let error = run(["mdsearch", "collection", "create", "notes"], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("the duplicate collection command should fail"))?;

    assert!(error.to_string().contains("already in use"));

    Ok(())
}

/// Covers: FR-005 and FR-012 — invalid names are rejected semantically.
#[rstest]
#[case("")]
#[case("   ")]
#[case("Notes/2026")]
#[case("Notes\\2026")]
#[case("Notes\n2026")]
fn rejects_invalid_collection_names(#[case] name: &str) -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;

    let error = run(["mdsearch", "collection", "create", name], home.path())
        .err()
        .ok_or_else(|| std::io::Error::other("an invalid collection name should fail"))?;

    assert!(error.to_string().contains("collection name"));
    assert!(!home.path().join(".mdsearch/collections.db").exists());

    Ok(())
}

/// Covers: FR-011 and FR-012 — database failures do not create partial state.
#[test]
fn reports_an_inaccessible_database_without_partial_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let blocked_parent = home.path().join("blocked");
    std::fs::write(&blocked_parent, "not a directory")?;
    let database_path = blocked_parent.join("collections.db");
    let database_argument = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            "--database",
            database_argument,
        ],
        home.path(),
    )
    .err()
    .ok_or_else(|| std::io::Error::other("the inaccessible database should fail"))?;

    assert!(error.to_string().contains("database"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: the single-binary success path and human-readable output.
#[test]
fn binary_reports_success_for_collection_creation() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "create", "Notes"])
        .env("HOME", home.path())
        .output()?;

    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("Notes"));

    Ok(())
}

/// Covers: the single-binary semantic failure path.
#[test]
fn binary_reports_invalid_name_failure() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "create", "Notes/2026"])
        .env("HOME", home.path())
        .output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)?.contains("collection name"));

    Ok(())
}

/// Covers: failure when the process has no home directory.
#[test]
fn binary_reports_missing_home_directory() -> Result<(), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "create", "Notes"])
        .env_remove("HOME")
        .output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)?.contains("home directory"));

    Ok(())
}
