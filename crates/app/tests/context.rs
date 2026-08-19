//! Acceptance tests for the `mdsearch context` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

fn store_and_update(
    home: &Path,
    collection: &str,
    files: &[(&str, &str)],
) -> Result<(), Box<dyn Error>> {
    let vault = home.join("vault");
    fs::create_dir_all(&vault)?;
    for (name, content) in files {
        fs::write(vault.join(name), content)?;
    }

    run(["mdsearch", "collection", "create", collection], home)?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            collection,
            path_argument(&vault)?,
        ],
        home,
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            collection,
            path_argument(&vault)?,
        ],
        home,
    )?;
    Ok(())
}

/// Covers: REQ-013 FR-006 — neighbors are returned as JSON.
#[test]
fn context_returns_neighbors_as_json() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(
        home.path(),
        "Notes",
        &[
            ("a.md", "---\n---\n[to](b.md)\n"),
            ("b.md", "---\n---\nbody\n"),
        ],
    )?;

    let a = home.path().join("vault").join("a.md");
    let query = format!(
        r#"{{ neighbors(collection: "Notes", kind: "file", key: "{}", maxHops: 2) {{ key relation depth }} }}"#,
        a.to_string_lossy()
    );
    let output = run(
        ["mdsearch", "context", &query, "--collection", "Notes"],
        home.path(),
    )?;

    let value: serde_json::Value = serde_json::from_str(&output)?;
    let neighbors = value
        .get("neighbors")
        .and_then(|neighbors| neighbors.as_array())
        .ok_or("expected neighbors array")?;
    assert!(!neighbors.is_empty());
    assert!(neighbors.iter().any(|entry| {
        entry.get("relation").and_then(|relation| relation.as_str()) == Some("LINKS_TO")
            && entry.get("depth").and_then(serde_json::Value::as_u64) == Some(1)
    }));

    Ok(())
}

/// Covers: REQ-013 FR-006 — node lookup returns JSON.
#[test]
fn context_returns_node_lookup() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(home.path(), "Notes", &[("a.md", "---\n---\nbody\n")])?;

    let a = home.path().join("vault").join("a.md");
    let query = format!(
        r#"{{ node(collection: "Notes", kind: "file", key: "{}") {{ key title }} }}"#,
        a.to_string_lossy()
    );
    let output = run(
        ["mdsearch", "context", &query, "--collection", "Notes"],
        home.path(),
    )?;

    let value: serde_json::Value = serde_json::from_str(&output)?;
    let node = value.get("node").ok_or("expected a node")?;
    assert_eq!(
        node.get("key").and_then(|key| key.as_str()),
        Some(a.to_str().unwrap_or(""))
    );
    assert_eq!(
        node.get("title").and_then(|title| title.as_str()),
        Some("a.md")
    );

    Ok(())
}

/// Covers: REQ-013 FR-007 — the command requires --collection.
#[test]
fn context_requires_a_collection() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let query = r#"{ node(collection: "Notes", kind: "file", key: "a.md") { key } }"#;
    let result = run(["mdsearch", "context", query], home.path());
    assert!(result.is_err());
    Ok(())
}

/// Covers: REQ-013 FR-008 — an unknown node is reported as an error.
#[test]
fn context_reports_unknown_node() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(home.path(), "Notes", &[("a.md", "---\n---\nbody\n")])?;

    let query = r#"{ node(collection: "Notes", kind: "file", key: "zzz.md") { key } }"#;
    let result = run(
        ["mdsearch", "context", query, "--collection", "Notes"],
        home.path(),
    );
    assert!(result.is_err());
    let message = format!("{result:?}");
    assert!(message.contains("node not found"));
    Ok(())
}

/// Covers: REQ-013 FR-009 — a malformed query is reported as an error.
#[test]
fn context_rejects_a_malformed_query() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(home.path(), "Notes", &[("a.md", "---\n---\nbody\n")])?;

    let result = run(
        [
            "mdsearch",
            "context",
            "not graphql",
            "--collection",
            "Notes",
        ],
        home.path(),
    );
    assert!(result.is_err());
    Ok(())
}

/// Covers: REQ-013 FR-010 — a missing database fails without creating a file.
#[test]
fn context_reports_missing_database_without_creating_one() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let missing = home.path().join("missing").join("collections.db");
    let query = r#"{ node(collection: "Notes", kind: "file", key: "a.md") { key } }"#;
    let result = run(
        [
            "mdsearch",
            "context",
            query,
            "--collection",
            "Notes",
            "--database",
            path_argument(&missing)?,
        ],
        home.path(),
    );
    assert!(result.is_err());
    assert!(!missing.exists());
    Ok(())
}
