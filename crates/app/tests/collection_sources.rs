//! Acceptance tests for collection source registration and listing.

use std::error::Error;
use std::fs;

use tempfile::tempdir;

mod common;
use common::run;

/// Covers: REQ-021 FR-001, FR-013 — registration is canonical and does not index.
#[test]
fn create_registers_canonical_sources_without_indexing_content() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir(&vault)?;
    fs::write(vault.join("notes.md"), "not indexed yet")?;
    let relative = home.path().join("vault");
    let source = relative.to_str().ok_or("path")?;

    run(
        ["mdsearch", "collection", "create", "Notes", source],
        home.path(),
    )?;
    let error = run(
        ["mdsearch", "get", "notes.md", "--collection", "Notes"],
        home.path(),
    )
    .err()
    .ok_or("source registration must not index content")?;

    assert!(error.to_string().contains("file not found"));

    Ok(())
}

/// Covers: REQ-021 FR-002 — configuration replaces roots and list JSON exposes them.
#[test]
fn configure_replaces_sources_and_json_list_reports_canonical_paths() -> Result<(), Box<dyn Error>>
{
    let home = tempdir()?;
    let first = home.path().join("first");
    let second = home.path().join("second");
    fs::create_dir(&first)?;
    fs::create_dir(&second)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            first.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;
    let configure_error = run(
        [
            "mdsearch",
            "collection",
            "configure",
            "Notes",
            "--sources",
            home.path().join("missing").to_str().ok_or("path")?,
        ],
        home.path(),
    )
    .err()
    .ok_or("a missing source should fail configuration")?;
    assert!(configure_error.to_string().contains("unreadable"));
    let before_replacement = run(["mdsearch", "collection", "list", "--json"], home.path())?;
    let unchanged: serde_json::Value = serde_json::from_str(&before_replacement)?;
    let unchanged_path = unchanged
        .get("collections")
        .and_then(serde_json::Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("sources"))
        .and_then(serde_json::Value::as_array)
        .and_then(|sources| sources.first())
        .ok_or("expected the previous source to remain")?;
    assert_eq!(
        unchanged_path,
        first.canonicalize()?.to_str().ok_or("path")?
    );
    run(
        [
            "mdsearch",
            "collection",
            "configure",
            "Notes",
            "--sources",
            second.to_str().ok_or("path")?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "collection", "list", "--json"], home.path())?;
    let json: serde_json::Value = serde_json::from_str(&output)?;
    let entry = json
        .get("collections")
        .and_then(serde_json::Value::as_array)
        .and_then(|entries| entries.first())
        .ok_or("expected one collection")?;
    assert_eq!(
        entry.get("name"),
        Some(&serde_json::Value::String("Notes".to_owned()))
    );
    let human = run(["mdsearch", "collection", "list"], home.path())?;
    assert!(human.contains(&second.canonicalize()?.display().to_string()));
    assert_eq!(
        entry
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .and_then(|sources| sources.first())
            .ok_or("expected a source")?,
        second.canonicalize()?.to_str().ok_or("path")?
    );

    Ok(())
}
