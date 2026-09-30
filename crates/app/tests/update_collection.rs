//! Acceptance tests for registered-source collection updates.

use std::error::Error;
use std::fs;
use std::process::Command;

use tempfile::tempdir;

use kv_app::run;
use kv_application::FileRecord;
use kv_application::{CollectionStore, FileStore};
use kv_domain::{CollectionName, Timestamp};
use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore};

fn update(home: &std::path::Path, name: &str) -> Result<String, kv_app::AppError> {
    run(
        ["mdsearch", "collection", "update", "--collection", name],
        home,
    )
}

/// Covers: REQ-021 FR-004, FR-006 — updates discover newly added files from saved sources.
#[test]
fn discovers_additions_and_modifications_from_registered_roots() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("a.md"), "alpha")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            vault.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;

    let first = update(home.path(), "Notes")?;
    assert!(first.contains("added 1, modified 0, deleted 0"));
    fs::write(vault.join("a.md"), "edited alpha")?;
    let modified = update(home.path(), "Notes")?;
    assert!(modified.contains("added 0, modified 1, deleted 0"));
    fs::write(vault.join("b.md"), "beta")?;
    let second = update(home.path(), "Notes")?;
    assert!(second.contains("added 1, modified 0, deleted 0"));

    Ok(())
}

/// Covers: REQ-021 FR-005, FR-007 — overlapping sources deduplicate and configure removes exclusive files on update.
#[test]
fn deduplicates_overlapping_sources_and_removes_files_after_reconfiguration()
-> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let root = home.path().join("vault");
    let nested = root.join("nested");
    fs::create_dir_all(&nested)?;
    fs::write(nested.join("a.md"), "alpha")?;
    let root_arg = root.to_str().ok_or("path")?;
    let nested_arg = nested.to_str().ok_or("path")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            root_arg,
            nested_arg,
        ],
        home.path(),
    )?;
    let initial = update(home.path(), "Notes")?;
    assert!(initial.contains("added 1, modified 0, deleted 0"));

    let configure_error = run(
        [
            "mdsearch",
            "collection",
            "configure",
            "Notes",
            "--sources",
            home.path().join("empty").to_str().ok_or("path")?,
        ],
        home.path(),
    )
    .err()
    .ok_or("a missing source directory should fail configuration")?;
    assert!(configure_error.to_string().contains("unreadable"));
    fs::create_dir(home.path().join("empty"))?;
    run(
        [
            "mdsearch",
            "collection",
            "configure",
            "Notes",
            "--sources",
            home.path().join("empty").to_str().ok_or("path")?,
        ],
        home.path(),
    )?;
    let reconciled = update(home.path(), "Notes")?;
    assert!(reconciled.contains("added 0, modified 0, deleted 1"));

    Ok(())
}

/// Covers: REQ-021 FR-012 — legacy source-less collections receive actionable update guidance.
#[test]
fn source_less_collections_require_explicit_registration() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database = home.path().join(".mdsearch/collections.db");
    let collection = CollectionName::try_from("Legacy")?;
    SqliteCollectionStore::open(&database)?
        .create_collection(&collection, Timestamp::from_unix_seconds(1_700_000_000))?;
    let file = home.path().join("legacy.md");
    fs::write(&file, "searchable legacy content")?;
    SqliteFileStore::open_for_ingestion(&database)?.reconcile(
        &collection,
        &[FileRecord::new(file, b"searchable legacy content".to_vec())],
        &[],
        Timestamp::from_unix_seconds(1_700_000_000),
    )?;

    let error = update(home.path(), "Legacy")
        .err()
        .ok_or("expected update failure")?;
    assert!(error.to_string().contains("configure sources"));
    let search = run(
        ["mdsearch", "search", "searchable", "--collection", "Legacy"],
        home.path(),
    )?;
    assert!(search.contains("legacy.md"));

    Ok(())
}

/// Covers: REQ-021 FR-011 — update-all continues and reports each collection after failures.
#[test]
fn update_all_reports_all_collections_when_one_has_no_sources() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let source = home.path().join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("a.md"), "alpha")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Configured",
            source.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;
    run(["mdsearch", "collection", "create", "Legacy"], home.path())?;

    let error = run(["mdsearch", "collection", "update", "--all"], home.path())
        .err()
        .ok_or("expected partial failure")?;
    let report = error.to_string();
    assert!(report.contains("Configured"));
    assert!(report.contains("Legacy"));
    assert!(report.contains("added 1"));

    Ok(())
}

/// Covers: REQ-021 FR-008 — an inaccessible directory aborts without losing the prior lexical index.
#[test]
fn inaccessible_source_preserves_the_last_committed_index() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let source = home.path().join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("a.md"), "retainedword")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            source.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;
    update(home.path(), "Notes")?;
    fs::remove_dir_all(&source)?;

    let error = update(home.path(), "Notes")
        .err()
        .ok_or("expected inaccessible source error")?;
    assert!(error.to_string().contains("unreadable"));
    let search = run(
        [
            "mdsearch",
            "search",
            "retainedword",
            "--collection",
            "Notes",
        ],
        home.path(),
    )?;
    assert!(search.contains("a.md"));

    Ok(())
}

/// Covers: REQ-021 FR-011 — the executable exits unsuccessfully after reporting every update-all outcome.
#[test]
fn update_all_binary_reports_partial_failure_and_nonzero_exit() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let source = home.path().join("source");
    fs::create_dir(&source)?;
    fs::write(source.join("a.md"), "alpha")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Configured",
            source.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;
    run(["mdsearch", "collection", "create", "Legacy"], home.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "update", "--all"])
        .env("HOME", home.path())
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!output.status.success());
    assert!(stderr.contains("Configured"));
    assert!(stderr.contains("Legacy"));
    assert!(stderr.contains("configure sources"));

    Ok(())
}
