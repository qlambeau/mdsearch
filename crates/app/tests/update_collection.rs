//! Acceptance tests for the `mdsearch collection update` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

/// Covers: FR-006 and FR-015 — a new file is reported as added.
#[test]
fn updates_reports_added_files() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;

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

    fs::write(vault.join("b.md"), "beta")?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    assert_eq!(
        output,
        "updated collection \"Notes\": added 1, modified 0, deleted 0"
    );

    Ok(())
}

/// Covers: FR-007 — a modified file is reported.
#[test]
fn updates_reports_modified_files() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;

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

    fs::write(vault.join("a.md"), "changed")?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    assert_eq!(
        output,
        "updated collection \"Notes\": added 0, modified 1, deleted 0"
    );

    Ok(())
}

/// Covers: FR-008 — a deleted file is reported.
#[test]
fn updates_reports_deleted_files() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;

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

    fs::remove_file(vault.join("a.md"))?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    assert_eq!(
        output,
        "updated collection \"Notes\": added 0, modified 0, deleted 1"
    );

    Ok(())
}

/// Covers: FR-009 — an unchanged collection reports zero changes.
#[test]
fn updates_reports_no_changes_for_an_unchanged_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;

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

    let output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;

    assert_eq!(
        output,
        "updated collection \"Notes\": added 0, modified 0, deleted 0"
    );

    Ok(())
}

/// Covers: FR-010 — `--all` emits one line per collection.
#[test]
fn updates_all_collections() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    fs::write(&a, "alpha")?;
    fs::write(&b, "beta")?;

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(["mdsearch", "collection", "create", "Archive"], home.path())?;
    run(
        ["mdsearch", "collection", "add", "Notes", path_argument(&a)?],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Archive",
            path_argument(&b)?,
        ],
        home.path(),
    )?;

    fs::write(&a, "edited")?;

    let output = run(["mdsearch", "collection", "update", "--all"], home.path())?;

    let lines = output.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("\"Archive\": added 0, modified 0, deleted 0"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("\"Notes\": added 0, modified 1, deleted 0"))
    );

    Ok(())
}

/// Covers: FR-011 — an unreadable path fails.
#[test]
fn updates_fails_for_an_unreadable_path() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    let missing = home.path().join("missing.md");

    let error = run(
        [
            "mdsearch",
            "collection",
            "update",
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

/// Covers: FR-012 — `--force` skips unreadable paths.
#[test]
fn updates_skips_unreadable_paths_with_force() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;
    let missing = home.path().join("missing.md");

    run(["mdsearch", "collection", "create", "Notes"], home.path())?;

    let output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&missing)?,
            path_argument(&vault)?,
            "--force",
        ],
        home.path(),
    )?;

    assert_eq!(
        output,
        "updated collection \"Notes\": added 1, modified 0, deleted 0 (skipped 1)"
    );

    Ok(())
}

/// Covers: FR-013 — a missing collection fails.
#[test]
fn updates_reports_a_missing_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    fs::write(&file, "alpha")?;
    run(["mdsearch", "collection", "create", "Other"], home.path())?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "update",
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

/// Covers: FR-014 — a missing database fails without creation.
#[test]
fn updates_reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    fs::write(&file, "alpha")?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "collection",
            "update",
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
