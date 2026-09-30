//! Process-level checks for CLI help and option spellings.

use std::error::Error;
use std::path::Path;
use std::process::Command;

use rstest::rstest;
use tempfile::tempdir;

use kv_app::run;

/// Covers: REQ-020 FR-006/FR-007 — short switches select the requested search scope and limit.
#[test]
fn search_short_switches_select_scope_and_limit() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    let notes = directory.path().join("notes.md");
    let other = directory.path().join("other.md");
    std::fs::write(&notes, "rust notes\n\nrust ownership\n")?;
    std::fs::write(&other, "rust other\n\nrust language\n")?;
    create_indexed_collection(directory.path(), database, "Notes", &notes)?;
    create_indexed_collection(directory.path(), database, "Other", &other)?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["search", "rust", "-c", "Notes", "-n", "1", "--json"])
        .arg("--database")
        .arg(database)
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json.get("scope"), Some(&serde_json::json!("Notes")));
    assert_eq!(json.get("limit"), Some(&serde_json::json!(1)));
    assert_eq!(
        json.get("results")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    Ok(())
}

/// Covers: REQ-020 FR-007 — the default search limit is ten.
#[test]
fn search_uses_default_limit_of_ten() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    let file = directory.path().join("many.md");
    let content = (0..12)
        .map(|index| format!("rust passage {index}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    std::fs::write(&file, content)?;
    create_indexed_collection(directory.path(), database, "Notes", &file)?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["search", "rust", "--json", "--database"])
        .arg(database)
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json.get("limit"), Some(&serde_json::json!(10)));
    assert_eq!(
        json.get("results")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(10)
    );

    Ok(())
}

/// Covers: REQ-020 FR-007 — search accepts only limits in the inclusive range 1–100.
#[rstest]
#[case("1", 1)]
#[case("100", 1)]
#[case("0", 2)]
#[case("101", 2)]
fn search_limit_boundaries_are_enforced(
    #[case] limit: &str,
    #[case] expected_exit: i32,
) -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing.db");
    let database = path_argument(&database_path)?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["search", "rust", "-n", limit, "--database", database])
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(expected_exit));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());

    Ok(())
}

/// Covers: REQ-020 FR-008 — every command depth has descriptive help with examples.
#[rstest]
#[case(&["--help"])]
#[case(&["collection", "--help"])]
#[case(&["collection", "create", "--help"])]
#[case(&["collection", "list", "--help"])]
#[case(&["collection", "destroy", "--help"])]
#[case(&["collection", "add", "--help"])]
#[case(&["collection", "update", "--help"])]
#[case(&["index", "--help"])]
#[case(&["index", "status", "--help"])]
#[case(&["search", "--help"])]
#[case(&["get", "--help"])]
#[case(&["embed", "--help"])]
#[case(&["hybrid", "--help"])]
#[case(&["graph", "neighbors", "--help"])]
#[case(&["graph", "--help"])]
#[case(&["context", "--help"])]
fn help_describes_command_and_shows_example(
    #[case] arguments: &[&str],
) -> Result<(), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(arguments)
        .env_remove("HOME")
        .output()?;
    let help = String::from_utf8(output.stdout)?;

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(help.to_lowercase().contains("example"), "{help}");
    assert!(help.lines().any(|line| line.starts_with("  ")), "{help}");

    Ok(())
}

/// Covers: REQ-020 FR-009 — ingestion accepts `--skip-unreadable`.
#[rstest]
#[case("add")]
fn ingestion_accepts_skip_unreadable(#[case] subcommand: &str) -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            "--database",
            database,
        ],
        directory.path(),
    )?;
    let readable = directory.path().join("readable.md");
    std::fs::write(&readable, "rust note")?;
    let readable = path_argument(&readable)?;
    let unreadable_path = directory.path().join("missing.md");
    let unreadable = path_argument(&unreadable_path)?;
    let arguments = [
        "collection",
        subcommand,
        "Notes",
        readable,
        unreadable,
        "--database",
        database,
        "--skip-unreadable",
    ];
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(arguments)
        .env("HOME", directory.path())
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8(output.stdout)?.contains("skipped 1"));

    Ok(())
}

/// Covers: REQ-021 FR-009 — source-aware update accepts the skip option.
#[test]
fn update_accepts_skip_unreadable() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    let source = directory.path().join("source");
    std::fs::create_dir(&source)?;
    std::fs::write(source.join("readable.md"), "rust note")?;
    let source = path_argument(&source)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            source,
            "--database",
            database,
        ],
        directory.path(),
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args([
            "collection",
            "update",
            "--collection",
            "Notes",
            "--database",
            database,
            "--skip-unreadable",
        ])
        .env("HOME", directory.path())
        .output()?;

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8(output.stdout)?.contains("added 1"));
    Ok(())
}

/// Covers: REQ-020 FR-009 — the misleading `--force` switch is rejected.
#[test]
fn ingestion_rejects_force_spelling() -> Result<(), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["collection", "add", "Notes", "missing.md", "--force"])
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("--force"));

    Ok(())
}

/// Covers: REQ-020 FR-005 — explicit database and cache paths need no HOME.
#[test]
fn explicit_paths_reach_model_check_without_home() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            "--semantic",
            "--database",
            database,
        ],
        directory.path(),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["embed", "-c", "Notes", "--database", database])
        .env_remove("HOME")
        .env("HF_HOME", directory.path().join("models"))
        .env_remove("FASTEMBED_CACHE_DIR")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr)?;
    assert!(diagnostic.contains("not available locally"));
    assert!(!diagnostic.contains("home directory"));

    Ok(())
}

/// Covers: REQ-020 FR-006 — graph collection filters accept `-c`.
#[test]
fn graph_collection_filter_accepts_short_switch() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing.db");
    let database = path_argument(&database_path)?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args([
            "graph",
            "neighbors",
            "rust.md",
            "-c",
            "Notes",
            "--database",
            database,
        ])
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("database"));

    Ok(())
}

/// Covers: REQ-020 FR-006 — context collection selection accepts `-c`.
#[test]
fn context_collection_filter_accepts_short_switch() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing.db");
    let database = path_argument(&database_path)?;
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(["context", "{}", "-c", "Notes", "--database", database])
        .env_remove("HOME")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("database"));

    Ok(())
}

/// Covers: REQ-020 FR-006 — hybrid search accepts both short option spellings.
#[test]
fn hybrid_accepts_short_collection_and_limit_switches() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("collections.db");
    let database = path_argument(&database_path)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            "--database",
            database,
        ],
        directory.path(),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args([
            "hybrid",
            "rust",
            "-c",
            "Notes",
            "-n",
            "1",
            "--database",
            database,
        ])
        .env_remove("HOME")
        .env("HF_HOME", directory.path().join("models"))
        .env_remove("FASTEMBED_CACHE_DIR")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8(output.stderr)?.contains("unexpected argument"));

    Ok(())
}

fn create_indexed_collection(
    home: &Path,
    database: &str,
    collection: &str,
    file: &Path,
) -> Result<(), Box<dyn Error>> {
    let file = path_argument(file)?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            collection,
            file,
            "--database",
            database,
        ],
        home,
    )?;
    run(
        [
            "mdsearch",
            "collection",
            "update",
            "--collection",
            collection,
            "--database",
            database,
        ],
        home,
    )?;

    Ok(())
}

fn path_argument(path: &Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("temporary test path should be UTF-8"))
}

/// Covers: REQ-020 FR-010 — the README describes the current switch and exit behavior.
#[test]
fn readme_documents_current_diagnostic_and_ingestion_contract() {
    let readme = include_str!("../../../README.md");

    assert!(readme.contains("`--skip-unreadable`"));
    assert!(!readme.contains("`--force`"));
    assert!(readme.contains("exit 0"));
    assert!(readme.contains("exit 1"));
    assert!(readme.contains("exit 2"));
}
