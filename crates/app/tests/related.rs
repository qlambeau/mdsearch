//! Acceptance tests for the `--related` switch on `search`/`hybrid`.

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

/// Covers: REQ-013 FR-001/FR-002 — --related lists file-to-file links in human
/// output, omitting tags.
#[test]
fn related_lists_file_to_file_links_in_human_output() -> Result<(), Box<dyn Error>> {
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
            "search",
            "rust",
            "--related",
            "--collection",
            "Notes",
        ],
        home.path(),
    )?;

    assert!(output.contains("(LINKS_TO)"));
    let b = home.path().join("vault").join("b.md");
    assert!(output.contains(&b.to_string_lossy().into_owned()));
    assert!(
        !output
            .lines()
            .any(|line| line.starts_with("related:") && line.contains("(TAGGED_WITH)"))
    );

    Ok(())
}

/// Covers: REQ-013 FR-003 — --related adds a related field to JSON output.
#[test]
fn related_adds_field_to_json_output() -> Result<(), Box<dyn Error>> {
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
            "search",
            "rust",
            "--related",
            "--json",
            "--collection",
            "Notes",
        ],
        home.path(),
    )?;

    let value: serde_json::Value = serde_json::from_str(&output)?;
    let results = value
        .get("results")
        .and_then(|results| results.as_array())
        .ok_or("expected results array")?;
    assert!(!results.is_empty());
    let related_entries = results
        .iter()
        .filter_map(|entry| entry.get("related").and_then(|related| related.as_array()))
        .flatten()
        .collect::<Vec<_>>();
    assert!(!related_entries.is_empty());
    assert!(related_entries.iter().any(|entry| {
        entry.get("relation").and_then(|relation| relation.as_str()) == Some("LINKS_TO")
    }));

    Ok(())
}

/// Covers: REQ-013 FR-004 — without --related the output has no related lines.
#[test]
fn output_is_unchanged_without_related_flag() -> Result<(), Box<dyn Error>> {
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
        ["mdsearch", "search", "rust", "--collection", "Notes"],
        home.path(),
    )?;

    assert!(!output.lines().any(|line| line.starts_with("related:")));

    Ok(())
}
