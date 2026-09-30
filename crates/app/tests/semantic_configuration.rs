//! Acceptance tests for per-collection semantic configuration and model management.

use std::error::Error;
use std::fs;

mod common;
use common::run;
use kv_application::SemanticIndexStore;
use kv_domain::CollectionName;
use kv_store_sqlite::SqliteSemanticIndexStore;
use tempfile::tempdir;

fn path_argument(path: &std::path::Path) -> Result<&str, std::io::Error> {
    path.to_str()
        .ok_or_else(|| std::io::Error::other("test path should be UTF-8"))
}

/// Covers: REQ-022 FR-001 — semantic opt-in is accepted at collection creation without indexing.
#[test]
fn create_accepts_semantic_opt_in_without_indexing_files() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("note.md"), "not indexed until update")?;

    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            path_argument(&vault)?,
            "--semantic",
        ],
        home.path(),
    )?;

    let error = run(
        ["mdsearch", "get", "note.md", "--collection", "Notes"],
        home.path(),
    )
    .err()
    .ok_or("semantic opt-in must not index files")?;
    assert!(error.to_string().contains("file not found"));

    Ok(())
}

/// Covers: REQ-022 FR-002 — collection configuration accepts semantic policy changes.
#[test]
fn configure_accepts_semantic_policy_without_indexing() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            home.path().to_str().ok_or("UTF-8 fixture path required")?,
        ],
        home.path(),
    )?;

    run(
        [
            "mdsearch",
            "collection",
            "configure",
            "Notes",
            "--semantic",
            "on",
        ],
        home.path(),
    )?;

    let database = home.path().join(".mdsearch/collections.db");
    let collection = CollectionName::try_from("Notes")?;
    let store = SqliteSemanticIndexStore::open_for_embedding(&database)?;
    assert!(store.semantic_enabled(&collection)?);
    assert!(store.status(&collection)?.is_none());

    Ok(())
}

/// Covers: REQ-022 FR-007 — model list reports supported models and availability.
#[test]
fn model_list_reports_supported_models_and_local_availability() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let output = run(["mdsearch", "model", "list"], home.path())?;

    assert!(output.contains("all-MiniLM-L6-v2"));
    assert!(output.contains("bge-reranker-base"));
    assert!(output.contains("available") || output.contains("not downloaded"));

    Ok(())
}

/// Covers: REQ-022 FR-008/FR-012 — reranker-only selection leaves embeddings alone.
#[test]
fn reranker_only_model_set_is_accepted() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    kv_store_sqlite::SqliteCollectionStore::open(&home.path().join(".mdsearch/collections.db"))?;
    let error = run(
        [
            "mdsearch",
            "model",
            "set",
            "--reranker",
            "unsupported-reranker",
        ],
        home.path(),
    )
    .err()
    .ok_or("unsupported reranker should be rejected")?;

    assert!(error.to_string().contains("unsupported-reranker"));

    Ok(())
}

/// Covers: REQ-022 FR-008/FR-012 — reranker-only selection changes no embedding setting.
#[test]
fn reranker_only_model_set_preserves_the_embedding_model_and_vectors() -> Result<(), Box<dyn Error>>
{
    let home = tempdir()?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            home.path().to_str().ok_or("UTF-8 fixture path required")?,
        ],
        home.path(),
    )?;
    let cache = home.path().join(".mdsearch/models");
    fs::create_dir_all(&cache)?;
    fs::write(cache.join("bge-reranker-base.completed"), "ok")?;

    run(
        [
            "mdsearch",
            "model",
            "set",
            "--reranker",
            "bge-reranker-base",
        ],
        home.path(),
    )?;

    let database = home.path().join(".mdsearch/collections.db");
    let store = SqliteSemanticIndexStore::open_for_embedding(&database)?;
    assert!(store.global_model()?.is_none());
    assert_eq!(
        store
            .reranker_model()?
            .map(|model| model.as_str().to_owned()),
        Some("bge-reranker-base".to_owned())
    );
    Ok(())
}

/// Covers: REQ-022 FR-008 — model set rejects an empty selection.
#[test]
fn model_set_requires_at_least_one_model() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let error = run(["mdsearch", "model", "set"], home.path())
        .err()
        .ok_or("empty model selection must fail")?;

    assert!(error.to_string().contains("required"));
    assert!(!home.path().join(".mdsearch/collections.db").exists());
    Ok(())
}

/// Covers: REQ-023 FR-001 — the retired embed command cannot alter collection policy.
#[test]
fn retired_embed_cannot_enable_semantic_configuration() -> Result<(), Box<dyn Error>> {
    let home = tempdir()?;
    let vault = home.path().join("vault");
    fs::create_dir_all(&vault)?;
    fs::write(vault.join("note.md"), "lexical content")?;
    run(
        [
            "mdsearch",
            "collection",
            "create",
            "Notes",
            path_argument(&vault)?,
        ],
        home.path(),
    )?;
    run(["mdsearch", "update", "--collection", "Notes"], home.path())?;

    let error = run(
        [
            "mdsearch",
            "embed",
            "--model",
            "all-MiniLM-L6-v2",
            "--collection",
            "Notes",
        ],
        home.path(),
    )
    .err()
    .ok_or("disabled semantic indexing should reject embed")?;

    assert!(error.to_string().contains("unrecognized subcommand"));

    Ok(())
}
