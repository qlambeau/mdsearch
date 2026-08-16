//! Integration tests for the system filesystem adapter.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use kv_application::{FileSystem, FileSystemError};
use kv_infrastructure::SystemFileSystem;
use tempfile::tempdir;

fn canonical(path: &Path) -> Result<PathBuf, std::io::Error> {
    fs::canonicalize(path)
}

/// Covers: FR-007 and FR-008 — directories are walked recursively and only
/// `.md` files are discovered, case-insensitively.
#[test]
fn expands_a_directory_recursively_for_markdown_files() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let vault = directory.path().join("vault");
    fs::create_dir_all(vault.join("sub"))?;
    fs::write(vault.join("a.md"), "alpha")?;
    fs::write(vault.join("sub").join("b.md"), "beta")?;
    fs::write(vault.join("sub").join("c.MD"), "gamma")?;
    fs::write(vault.join("readme.txt"), "not markdown")?;

    let mut expanded = SystemFileSystem.expand(&vault)?;
    expanded.sort();

    let mut expected = vec![
        canonical(&vault.join("a.md"))?,
        canonical(&vault.join("sub").join("b.md"))?,
        canonical(&vault.join("sub").join("c.MD"))?,
    ];
    expected.sort();

    assert_eq!(expanded, expected);

    Ok(())
}

/// Covers: FR-007 — a single markdown file expands to its canonical path.
#[test]
fn expands_a_single_markdown_file() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let file = directory.path().join("notes.md");
    fs::write(&file, "content")?;

    let expanded = SystemFileSystem.expand(&file)?;

    assert_eq!(expanded, vec![canonical(&file)?]);

    Ok(())
}

/// Covers: FR-008 — non-`.md` files are ignored.
#[test]
fn ignores_non_markdown_files() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let file = directory.path().join("notes.txt");
    fs::write(&file, "content")?;

    let expanded = SystemFileSystem.expand(&file)?;

    assert!(expanded.is_empty());

    Ok(())
}

/// Covers: FR-011 — an unreadable path reports a semantic error.
#[test]
fn reports_an_unreadable_path() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let missing = directory.path().join("missing.md");

    let error = SystemFileSystem
        .expand(&missing)
        .err()
        .ok_or_else(|| std::io::Error::other("a missing path should fail"))?;

    assert!(matches!(error, FileSystemError::Unreadable { .. }));

    Ok(())
}

/// Covers: reading file bytes.
#[test]
fn reads_file_bytes() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let file = directory.path().join("notes.md");
    fs::write(&file, b"content")?;

    let bytes = SystemFileSystem.read(&file)?;

    assert_eq!(bytes, b"content".to_vec());

    Ok(())
}

/// Covers: `exists` reports true for a present path.
#[test]
fn exists_reports_true_for_a_present_path() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let file = directory.path().join("notes.md");
    fs::write(&file, "content")?;

    assert!(SystemFileSystem.exists(&file)?);

    Ok(())
}

/// Covers: `exists` reports false only for a missing path.
#[test]
fn exists_reports_false_for_a_missing_path() -> Result<(), Box<dyn Error>> {
    let directory = tempdir()?;
    let missing = directory.path().join("missing.md");

    assert!(!SystemFileSystem.exists(&missing)?);

    Ok(())
}
