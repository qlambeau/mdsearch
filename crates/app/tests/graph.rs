//! Acceptance tests for the `mdsearch graph neighbors` debug command.

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

/// Covers: FR-014 — graph neighbors lists a node's neighbors with relation and
/// depth.
#[test]
fn graph_neighbors_lists_relationships() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(
        home.path(),
        "Notes",
        &[
            ("a.md", "---\ntags: [rust]\n---\n[to](b.md)\n"),
            ("b.md", "---\ntags: [rust]\n---\nbody\n"),
        ],
    )?;

    let output = run(
        [
            "mdsearch",
            "graph",
            "neighbors",
            path_argument(&home.path().join("vault").join("a.md"))?,
            "--collection",
            "Notes",
        ],
        home.path(),
    )?;

    assert!(output.contains("LINKS_TO"));
    assert!(output.contains("TAGGED_WITH"));
    assert!(output.contains("depth 1"));

    Ok(())
}

/// Covers: FR-014 — graph neighbors reports an unknown node as an error.
#[test]
fn graph_neighbors_reports_unknown_node() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    store_and_update(home.path(), "Notes", &[("a.md", "body\n")])?;

    let result = run(
        [
            "mdsearch",
            "graph",
            "neighbors",
            "does-not-exist.md",
            "--collection",
            "Notes",
        ],
        home.path(),
    );

    assert!(result.is_err());

    Ok(())
}

/// Covers: missing database — the command reports the database does not exist.
#[test]
fn graph_neighbors_reports_missing_database() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let missing = home.path().join("missing").join("collections.db");
    let result = run(
        [
            "mdsearch",
            "graph",
            "neighbors",
            "a.md",
            "--database",
            path_argument(&missing)?,
        ],
        home.path(),
    );

    assert!(result.is_err());

    Ok(())
}
