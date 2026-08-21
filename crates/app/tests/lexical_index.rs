//! Acceptance tests for the lexical index build and `index status` command.

use std::error::Error;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use kv_app::run;
use kv_application::SemanticIndexStore;
use kv_domain::{CollectionName, Embedding, EmbeddingModel, SemanticPassage, Timestamp};
use kv_store_sqlite::SqliteSemanticIndexStore;

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("the test path should be UTF-8"))
}

fn create_and_add(home: &Path, file: &Path, content: &str) -> Result<(), Box<dyn Error>> {
    fs::write(file, content)?;
    run(["mdsearch", "collection", "create", "Notes"], home)?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(file)?,
        ],
        home,
    )?;
    Ok(())
}

/// Builds a semantic index for "Notes" with fake vectors at `dimension`.
fn embed_with_dimension(home: &Path, dimension: usize) -> Result<(), Box<dyn Error>> {
    let database_path = home.join(".mdsearch").join("collections.db");
    let notes = CollectionName::try_from("Notes")?;
    let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    store.ensure_dimension(dimension)?;
    let passages = store.passages(&notes)?;
    let pairs = passages
        .iter()
        .cloned()
        .map(|passage| (passage, Embedding::new(vec![0.1; dimension])))
        .collect::<Vec<(SemanticPassage, Embedding)>>();
    store.rebuild(
        &notes,
        &EmbeddingModel::try_new("bge-large-en-v1.5")?,
        Timestamp::from_unix_seconds(1_700_000_000),
        &pairs,
    )?;
    Ok(())
}

/// Covers: FR-002 — adding files alone does not build the index.
#[test]
fn adding_files_alone_does_not_build_the_index() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    create_and_add(home.path(), &file, "alpha")?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert_eq!(output, "collection \"Notes\": lexical index not built");

    Ok(())
}

/// Covers: FR-001 and FR-009 — update builds the index and counts passages.
#[test]
fn update_builds_the_index_and_counts_passages() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    create_and_add(
        home.path(),
        &file,
        "---\ntitle: My Title\ntags: [rust]\n---\n\none\n\ntwo\n\nthree",
    )?;

    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        output.contains(
            "collection \"Notes\": lexical index built, 1 file(s), 5 passage(s), built at "
        ),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: FR-004 — every recognized frontmatter field is its own passage.
#[test]
fn every_recognized_frontmatter_field_is_its_own_passage() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("f.md");
    create_and_add(
        home.path(),
        &file,
        "---\ntitle: T\ntags: [a]\naliases: [b]\nsummary: S\n---\n\none\n\ntwo",
    )?;

    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        output.contains("lexical index built, 1 file(s), 6 passage(s)"),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: FR-008 — an edit refreshes the index and the build timestamp.
#[test]
fn update_refreshes_the_index_after_an_edit() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    create_and_add(home.path(), &file, "one\n\ntwo")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let first = run(["mdsearch", "index", "status"], home.path())?;
    assert!(
        first.contains("lexical index built, 1 file(s), 2 passage(s)"),
        "unexpected status: {first}"
    );

    fs::write(&file, "one\n\ntwo\n\nthree")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let second = run(["mdsearch", "index", "status"], home.path())?;
    assert!(
        second.contains("lexical index built, 1 file(s), 3 passage(s)"),
        "unexpected status: {second}"
    );

    Ok(())
}

/// Covers: FR-008 — a deleted file's passages are removed.
#[test]
fn update_removes_passages_of_a_deleted_file() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let a = home.path().join("a.md");
    let b = home.path().join("b.md");
    fs::write(&a, "alpha")?;
    fs::write(&b, "beta\n\ngamma")?;
    run(["mdsearch", "collection", "create", "Notes"], home.path())?;
    run(
        [
            "mdsearch",
            "collection",
            "add",
            "Notes",
            path_argument(&a)?,
            path_argument(&b)?,
        ],
        home.path(),
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&a)?,
            path_argument(&b)?,
        ],
        home.path(),
    )?;

    let before = run(["mdsearch", "index", "status"], home.path())?;
    assert!(
        before.contains("lexical index built, 2 file(s), 3 passage(s)"),
        "unexpected status: {before}"
    );

    fs::remove_file(&b)?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&a)?,
        ],
        home.path(),
    )?;

    let after = run(["mdsearch", "index", "status"], home.path())?;
    assert!(
        after.contains("lexical index built, 1 file(s), 1 passage(s)"),
        "unexpected status: {after}"
    );

    Ok(())
}

/// Covers: FR-006 — malformed frontmatter is reported and indexed body-only.
#[test]
fn malformed_frontmatter_is_indexed_body_only_and_reported() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("c.md");
    create_and_add(
        home.path(),
        &file,
        "---\ntitle: \"unterminated\n: bad: : :\n---\n\nbody",
    )?;

    let update_output = run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    assert!(
        update_output.contains("1 malformed frontmatter"),
        "unexpected update output: {update_output}"
    );

    let status_output = run(["mdsearch", "index", "status"], home.path())?;
    assert!(
        status_output.contains("lexical index built, 1 file(s), 1 passage(s)"),
        "unexpected status: {status_output}"
    );

    Ok(())
}

/// Covers: FR-005 — files without frontmatter are indexed by their body.
#[test]
fn files_without_frontmatter_are_indexed_by_their_body() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("d.md");
    create_and_add(home.path(), &file, "one\n\ntwo")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        output.contains("lexical index built, 1 file(s), 2 passage(s)"),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: FR-007 — empty files contribute no passages and the index is built.
#[test]
fn empty_files_contribute_no_passages() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("e.md");
    create_and_add(home.path(), &file, "")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        output.contains("lexical index built, 1 file(s), 0 passage(s)"),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: FR-017 — `update --all` rebuilds the index for every collection.
#[test]
fn update_all_rebuilds_the_index_for_every_collection() -> Result<(), Box<dyn Error>> {
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

    run(["mdsearch", "collection", "update", "--all"], home.path())?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    let lines = output.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("\"Archive\": lexical index built, 1 file(s), 1 passage(s)")),
        "unexpected status: {output}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("\"Notes\": lexical index built, 1 file(s), 1 passage(s)")),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: REQ-006 FR-017 — index status reports the recorded semantic model
/// and dimension for an embedded collection.
#[test]
fn index_status_reports_the_semantic_model_and_dimension() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    create_and_add(home.path(), &file, "one\n\ntwo")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;
    embed_with_dimension(home.path(), 1024)?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        output.contains("bge-large-en-v1.5 (1024 dimensions)"),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: REQ-006 FR-017 — index status reports nothing extra for a
/// collection without a semantic state row.
#[test]
fn index_status_reports_no_semantic_line_without_a_semantic_state() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let file = home.path().join("a.md");
    create_and_add(home.path(), &file, "one")?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "Notes",
            path_argument(&file)?,
        ],
        home.path(),
    )?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert!(
        !output.contains("dimensions"),
        "unexpected status: {output}"
    );
    assert!(
        output.contains("lexical index built"),
        "unexpected status: {output}"
    );

    Ok(())
}

/// Covers: FR-014 — a missing database fails without creating a file.
#[test]
fn index_status_reports_a_missing_database_without_creating_it() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join("custom").join("collections.db");
    let database_argument = path_argument(&database_path)?;

    let error = run(
        [
            "mdsearch",
            "index",
            "status",
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

/// Covers: FR-015 — a fresh database with no collections produces empty output.
#[test]
fn index_status_reports_empty_output_for_no_collections() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let database_path = home.path().join(".mdsearch").join("collections.db");
    kv_store_sqlite::SqliteCollectionStore::open(&database_path)?;

    let output = run(["mdsearch", "index", "status"], home.path())?;

    assert_eq!(output, "");

    Ok(())
}
