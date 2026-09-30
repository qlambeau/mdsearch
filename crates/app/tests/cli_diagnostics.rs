//! Process-level checks for CLI diagnostics and database selection.

use std::error::Error;
use std::process::{Command, Output};

use rstest::rstest;
use tempfile::tempdir;

fn run_without_home(arguments: &[&str]) -> Result<Output, Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .args(arguments)
        .env_remove("HOME")
        .env_remove("HF_HOME")
        .env_remove("FASTEMBED_CACHE_DIR")
        .output()?;

    Ok(output)
}

/// Covers: REQ-020 FR-001 — help and version succeed without resolving HOME.
#[rstest]
#[case(&["--help"])]
#[case(&["--version"])]
#[case(&["collection", "--help"])]
#[case(&["update", "--help"])]
#[case(&["graph", "neighbors", "--help"])]
fn informational_commands_succeed_without_home(
    #[case] arguments: &[&str],
) -> Result<(), Box<dyn Error>> {
    let output = run_without_home(arguments)?;

    assert_eq!(output.status.code(), Some(0));
    assert!(!output.stdout.is_empty());
    assert!(output.stderr.is_empty());

    Ok(())
}

/// Covers: REQ-020 FR-002 — argument errors retain clap's exit code without HOME.
#[test]
fn invalid_arguments_exit_two_without_home() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(&["--not-an-option"])?;

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("unexpected argument"));

    Ok(())
}

/// Covers: REQ-020 FR-003 — operational failures use exit code one.
#[test]
fn missing_database_is_an_operational_failure() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("missing.db");
    let database = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("temporary database path should be UTF-8"))?;
    let output = run_without_home(&["collection", "list", "--database", database])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("database"));
    assert!(!database_path.exists());

    Ok(())
}

/// Covers: REQ-020 FR-003/FR-005 — HOME is required only for default paths.
#[test]
fn default_database_reports_missing_home() -> Result<(), Box<dyn Error>> {
    let output = run_without_home(&["collection", "create", "Notes", "."])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("home directory"));

    Ok(())
}

/// Covers: REQ-020 FR-004/FR-005 — the global database option selects the database.
#[rstest]
#[case(&["--database"], &["collection", "create", "Notes", "."])]
#[case(&["collection", "--database"], &["create", "Notes"])]
#[case(&["collection", "create", "Notes", "--database"], &[])]
fn global_database_option_selects_explicit_path(
    #[case] prefix: &[&str],
    #[case] suffix: &[&str],
) -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("custom.db");
    let database = database_path
        .to_str()
        .ok_or_else(|| std::io::Error::other("temporary database path should be UTF-8"))?;
    let arguments = prefix
        .iter()
        .copied()
        .chain([database])
        .chain(suffix.iter().copied())
        .chain([directory
            .path()
            .to_str()
            .ok_or("UTF-8 source path required")?])
        .collect::<Vec<_>>();
    let output = run_without_home(&arguments)?;

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8(output.stdout)?.contains("Notes"));
    assert!(output.stderr.is_empty());
    assert!(database_path.exists());

    Ok(())
}
