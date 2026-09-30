//! REQ-023 subprocess contracts for the final CLI.

use kv_application::{CollectionStore, FileRecord, FileStore};
use kv_domain::{CollectionName, Timestamp};
use kv_store_sqlite::{SqliteCollectionStore, SqliteFileStore};
use rstest::rstest;
use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn invoke(database: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .env_remove("HOME")
        .env_remove("HF_HOME")
        .env_remove("FASTEMBED_CACHE_DIR")
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
}

fn seed(database: &Path, path: &Path, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let name = CollectionName::try_from("Notes")?;
    let at = Timestamp::from_unix_seconds(42);
    SqliteCollectionStore::open(database)?.create_collection(&name, at)?;
    let mut files = SqliteFileStore::open_for_ingestion(database)?;
    files.upsert_files(
        &name,
        &[FileRecord::new(path.to_owned(), bytes.to_vec())],
        at,
    )?;
    files.reconcile(&name, &[], &[], at)?;
    Ok(())
}

/// Covers: REQ-023 FR-001 — obsolete commands are rejected before HOME lookup.
#[rstest]
#[case(&["collection", "add"])]
#[case(&["collection", "destroy"])]
#[case(&["collection", "update"])]
#[case(&["embed"])]
#[case(&["hybrid"])]
#[case(&["index", "status"])]
#[case(&["context"])]
fn obsolete_commands_exit_two(#[case] args: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = invoke(Path::new("/unused.db"), args)?;

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    Ok(())
}

/// Covers: REQ-023 FR-017 — get emits exact stored bytes by both selectors.
#[rstest]
#[case(b"alpha".as_slice())]
#[case(b"alpha\n".as_slice())]
#[case(b"".as_slice())]
#[case(b"\xff\x00\xfe".as_slice())]
fn get_preserves_every_byte(#[case] content: &[u8]) -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), content)?;

    let by_name = invoke(&database, &["get", "a.md", "-c", "Notes"])?;
    let by_id = invoke(&database, &["get", "--id", "1", "-c", "Notes"])?;

    assert!(by_name.status.success(), "{by_name:?}");
    assert_eq!(by_name.stdout, content);
    assert!(by_name.stderr.is_empty());
    assert!(by_id.status.success(), "{by_id:?}");
    assert_eq!(by_id.stdout, content);
    assert!(by_id.stderr.is_empty());
    Ok(())
}

/// Covers: REQ-023 FR-016 — numeric names are names, not implicit IDs.
#[test]
fn numeric_names_remain_names() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("999"), b"numeric name")?;

    let output = invoke(&database, &["get", "999", "-c", "Notes"])?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"numeric name");
    Ok(())
}

/// Covers: REQ-023 FR-002/005/011/016/019/020 — validate before any I/O.
#[rstest]
#[case(&["collection", "create", "Notes"])]
#[case(&["collection", "configure", "Notes"])]
#[case(&["update"])]
#[case(&["update", "--all", "-c", "Notes"])]
#[case(&["search", "alpha", "--no-rerank"])]
#[case(&["search", "alpha", "--mode", "lexical", "--no-rerank"])]
#[case(&["get", "a.md", "--id", "1", "-c", "Notes"])]
#[case(&["get", "-c", "Notes"])]
#[case(&["get", "--id", "0", "-c", "Notes"])]
#[case(&["graph", "neighbors", "a.md"])]
#[case(&["graph", "query", "{ node { key } }"])]
#[case(&["graph", "neighbors", "a.md", "-c", "Notes", "--depth", "0"])]
#[case(&["graph", "neighbors", "a.md", "-c", "Notes", "--depth", "256"])]
#[case(&["graph", "neighbors", "a.md", "-c", "Notes", "--kind", "guess"])]
fn invalid_selectors_exit_two(#[case] args: &[&str]) -> Result<(), Box<dyn Error>> {
    let output = invoke(Path::new("/unused.db"), args)?;

    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    Ok(())
}

/// Covers: REQ-023 FR-005/024 — lexical updates do not require a model cache.
#[test]
fn lexical_workflow_runs_without_home() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let source = dir.path().join("vault");
    fs::create_dir(&source)?;
    fs::write(source.join("a.md"), "borrowing rules")?;

    let create = invoke(
        &database,
        &[
            "collection",
            "create",
            "Notes",
            source.to_str().ok_or("UTF-8 fixture path required")?,
        ],
    )?;
    let update = invoke(&database, &["update", "-c", "Notes", "--json"])?;

    assert!(create.status.success(), "{create:?}");
    assert!(update.status.success(), "{update:?}");
    let value: serde_json::Value = serde_json::from_slice(&update.stdout)?;
    assert_eq!(json_field(&value, "/collections/0/added")?, 1);
    assert!(update.stderr.is_empty());
    Ok(())
}

/// Covers: REQ-023 FR-008/014/015 — unified queries preserve JSON and identity.
#[test]
fn search_modes_preserve_contracts_and_identity() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"borrowing rules")?;

    let quoted = invoke(
        &database,
        &["search", "borrowing rules", "-c", "Notes", "--json"],
    )?;
    let words = invoke(
        &database,
        &["search", "borrowing", "rules", "-c", "Notes", "--json"],
    )?;
    let value: serde_json::Value = serde_json::from_slice(&quoted.stdout)?;
    let other: serde_json::Value = serde_json::from_slice(&words.stdout)?;

    assert!(quoted.status.success(), "{quoted:?}");
    assert_eq!(value, other);
    assert_eq!(json_field(&value, "/mode")?, "lexical");
    assert_eq!(json_field(&value, "/results/0/file_id")?, 1);
    for field in ["collection", "path", "kind", "text", "score", "position"] {
        assert!(
            json_field(&value, "/results/0")?.get(field).is_some(),
            "missing {field}"
        );
    }
    let human = invoke(&database, &["search", "borrowing", "-c", "Notes"])?;
    assert!(String::from_utf8(human.stdout)?.contains("Notes, file_id 1"));
    Ok(())
}

/// Covers: REQ-023 FR-009/015 — hybrid mode preserves score fields and file identity.
#[test]
fn hybrid_mode_preserves_contract_and_identity() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"borrowing rules")?;
    let hybrid = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .env_remove("HOME")
        .env("FASTEMBED_CACHE_DIR", dir.path().join("cache"))
        .env_remove("HF_HOME")
        .arg("--database")
        .arg(&database)
        .args([
            "search",
            "borrowing",
            "--mode",
            "hybrid",
            "--no-rerank",
            "--json",
        ])
        .output()?;
    assert!(hybrid.status.success(), "{hybrid:?}");
    let value: serde_json::Value = serde_json::from_slice(&hybrid.stdout)?;
    assert_eq!(json_field(&value, "/mode")?, "hybrid");
    assert_eq!(json_field(&value, "/results/0/file_id")?, 1);
    for field in [
        "reranker_score",
        "fused_score",
        "bm25_score",
        "cosine_similarity",
        "ordering_score",
        "position",
    ] {
        assert!(
            json_field(&value, "/results/0")?.get(field).is_some(),
            "missing {field}"
        );
    }
    Ok(())
}

/// Covers: REQ-023 FR-014 — human empty results explicitly report no matches.
#[test]
fn empty_search_is_explicit() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"alpha")?;

    let output = invoke(&database, &["search", "zzznotaword"])?;

    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8(output.stdout)?.contains("no matches"));
    Ok(())
}

/// Covers: REQ-023 FR-006/007 — failures report every collection on stderr.
#[test]
fn update_all_json_reports_success_and_failure_on_stderr() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let good = dir.path().join("good");
    let bad = dir.path().join("bad");
    fs::create_dir(&good)?;
    fs::create_dir(&bad)?;
    fs::write(good.join("a.md"), "alpha")?;
    fs::write(bad.join("b.md"), "previous")?;
    create_and_update_two_sources(&database, &good, &bad)?;
    fs::write(good.join("a.md"), "modified")?;
    fs::rename(&bad, dir.path().join("moved"))?;

    let output = invoke(&database, &["update", "--all", "--json"])?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr)?;
    assert_eq!(
        json_field(&value, "/collections")?
            .as_array()
            .ok_or("array required")?
            .len(),
        2
    );
    assert_eq!(json_field(&value, "/collections/0/name")?, "Archive");
    assert_eq!(json_field(&value, "/collections/0/success")?, false);
    assert!(json_field(&value, "/collections/0/diagnostic")?.is_string());
    assert_eq!(json_field(&value, "/collections/1/modified")?, 1);
    let old = invoke(&database, &["get", "b.md", "-c", "Archive"])?;
    let new = invoke(&database, &["get", "a.md", "-c", "Notes"])?;
    assert_eq!(old.stdout, b"previous");
    assert_eq!(new.stdout, b"modified");
    Ok(())
}

/// Covers: REQ-023 FR-003 — listing includes enabled indexes without indexing.
#[test]
fn collection_list_exposes_enabled_indexes() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let output = invoke(
        &database,
        &[
            "collection",
            "create",
            "Notes",
            dir.path().to_str().ok_or("UTF-8 fixture path required")?,
            "--semantic",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");

    let output = invoke(&database, &["collection", "list", "--json"])?;

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_field(&value, "/collections/0/name")?, "Notes");
    assert_eq!(
        json_field(&value, "/collections/0/sources/0")?,
        dir.path().to_str().ok_or("UTF-8 fixture path required")?
    );
    assert_eq!(
        json_field(&value, "/collections/0/indexes")?,
        &serde_json::json!({"lexical":true,"graph":true,"semantic":true})
    );
    Ok(())
}

/// Covers: REQ-023 FR-019/021 — all graph roots share invocation scope.
#[test]
fn graph_queries_and_neighbors_are_scoped() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    let c = dir.path().join("c.md");
    seed_two_graphs(&database, &a, &b, &c)?;
    let key = serde_json::to_string(a.to_str().ok_or("UTF-8 fixture path required")?)?;
    let query = format!(
        "{{ one: node(kind: \"file\", key: {key}) {{ key }} two: neighbors(kind: \"file\", key: {key}, maxHops: 1) {{ key depth }} }}"
    );

    let output = invoke(&database, &["graph", "query", &query, "-c", "Notes"])?;

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_field(&value, "/one/key")?,
        a.to_str().ok_or("UTF-8 fixture path required")?
    );
    assert_eq!(
        json_field(&value, "/two/0/key")?,
        b.to_str().ok_or("UTF-8 fixture path required")?
    );
    assert_eq!(json_field(&value, "/two/0/depth")?, 1);
    let neighbors = invoke(
        &database,
        &[
            "graph",
            "neighbors",
            a.to_str().ok_or("UTF-8 fixture path required")?,
            "-c",
            "Notes",
        ],
    )?;
    assert!(neighbors.status.success(), "{neighbors:?}");
    let text = String::from_utf8(neighbors.stdout)?;
    assert!(text.contains(b.to_str().ok_or("UTF-8 fixture path required")?));
    assert!(!text.contains(c.to_str().ok_or("UTF-8 fixture path required")?));
    let deeper = invoke(
        &database,
        &[
            "graph",
            "neighbors",
            a.to_str().ok_or("UTF-8 fixture path required")?,
            "-c",
            "Notes",
            "--kind",
            "file",
            "--relation",
            "LINKS_TO",
            "--depth",
            "2",
        ],
    )?;
    assert!(deeper.status.success(), "{deeper:?}");
    assert!(
        String::from_utf8(deeper.stdout)?
            .contains(c.to_str().ok_or("UTF-8 fixture path required")?)
    );
    Ok(())
}

/// Covers: REQ-023 FR-021 — graph fields cannot override invocation scope.
#[rstest]
#[case("node", "")]
#[case("neighbors", ", maxHops: 1")]
fn graph_scope_override_is_rejected(
    #[case] field: &str,
    #[case] extra: &str,
) -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"alpha")?;
    let query = format!(
        "{{ {field}(collection: \"Archive\", kind: \"file\", key: \"a.md\"{extra}) {{ key }} }}"
    );

    let output = invoke(&database, &["graph", "query", &query, "-c", "Notes"])?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("Unknown argument \"collection\""));
    Ok(())
}

/// Covers: REQ-023 FR-022/023 — status measures stored content and retains builds.
#[test]
fn status_compares_stored_bytes_without_scanning_sources() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let file = dir.path().join("a.md");
    fs::write(&file, "alpha")?;
    seed(&database, &file, b"alpha")?;
    let configuration = invoke(
        &database,
        &[
            "collection",
            "configure",
            "Notes",
            "--sources",
            file.to_str().ok_or("UTF-8 fixture path required")?,
        ],
    )?;
    assert!(configuration.status.success(), "{configuration:?}");
    let before = fs::read(&database)?;

    let current = invoke(&database, &["status", "-c", "Notes", "--json"])?;

    assert!(current.status.success(), "{current:?}");
    let value: serde_json::Value = serde_json::from_slice(&current.stdout)?;
    assert_current_indexes(&value)?;
    assert_eq!(
        fs::read(&database)?,
        before,
        "status must not mutate the database"
    );
    fs::write(&file, "filesystem-only edit")?;
    fs::rename(&file, dir.path().join("unavailable.md"))?;
    let pending = invoke(&database, &["status", "--json"])?;
    assert!(pending.status.success(), "{pending:?}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&pending.stdout)?,
        value
    );
    let collection = CollectionName::try_from("Notes")?;
    SqliteFileStore::open_for_ingestion(&database)?.upsert_files(
        &collection,
        &[FileRecord::new(file, b"omega".to_vec())],
        Timestamp::from_unix_seconds(43),
    )?;
    let stale = invoke(&database, &["status", "--json"])?;
    assert!(stale.status.success(), "{stale:?}");
    let value: serde_json::Value = serde_json::from_slice(&stale.stdout)?;
    assert_stale_indexes(&value)?;
    Ok(())
}

/// Covers: REQ-023 FR-022 — enabled unbuilt indexes have no successful build.
#[test]
fn status_reports_unbuilt_and_disabled_indexes() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let create = invoke(
        &database,
        &[
            "collection",
            "create",
            "Notes",
            dir.path().to_str().ok_or("UTF-8 fixture path required")?,
            "--semantic",
        ],
    )?;
    assert!(create.status.success(), "{create:?}");

    let output = invoke(&database, &["status", "--json"])?;

    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    for kind in ["lexical", "graph", "semantic"] {
        assert_eq!(
            json_field(&value, &format!("/collections/0/indexes/{kind}/enabled"))?,
            true
        );
        assert_eq!(
            json_field(&value, &format!("/collections/0/indexes/{kind}/readiness"))?,
            "not_built"
        );
        assert!(json_field(&value, &format!("/collections/0/indexes/{kind}/built_at"))?.is_null());
    }
    let unknown = invoke(&database, &["status", "-c", "Archive"])?;
    assert_eq!(unknown.status.code(), Some(1));
    assert!(String::from_utf8(unknown.stderr)?.contains("collection not found"));
    Ok(())
}

/// Covers: REQ-023 FR-012 — hybrid prerequisites give a replacement recovery command.
#[test]
fn hybrid_prerequisite_recovery_uses_final_grammar() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let collection = CollectionName::try_from("Notes")?;
    SqliteCollectionStore::open(&database)?
        .create_collection(&collection, Timestamp::from_unix_seconds(42))?;

    let output = Command::new(env!("CARGO_BIN_EXE_mdsearch"))
        .env_remove("HOME")
        .env_remove("HF_HOME")
        .env("FASTEMBED_CACHE_DIR", dir.path().join("cache"))
        .arg("--database")
        .arg(&database)
        .args(["search", "borrowing", "--mode", "hybrid", "-c", "Notes"])
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr)?;
    assert!(diagnostic.contains("not built"));
    assert!(diagnostic.contains("mdsearch"));
    assert!(
        diagnostic.contains("update --collection 'Notes'"),
        "{diagnostic}"
    );
    assert!(!dir.path().join("cache").exists());
    Ok(())
}

/// Covers: REQ-023 FR-019/020 — explicit kinds and maximum depth select typed nodes.
#[rstest]
#[case("tag", "rust", "TAGGED_WITH")]
#[case("alias", "RustGuide", "ALIAS_OF")]
fn explicit_graph_kinds_and_depth_bound(
    #[case] kind: &str,
    #[case] key: &str,
    #[case] relation: &str,
) -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let file = dir.path().join("a.md");
    seed(
        &database,
        &file,
        b"---\ntags: [rust]\naliases: [RustGuide]\n---\nbody",
    )?;
    let output = invoke(
        &database,
        &[
            "graph",
            "neighbors",
            key,
            "-c",
            "Notes",
            "--kind",
            kind,
            "--relation",
            relation,
            "--depth",
            "255",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout)?;
    // Metadata edges run from files to tags/aliases; those nodes have no outgoing edges.
    assert_eq!(text, format!("{kind} {key}:\n"));
    let outgoing = invoke(
        &database,
        &[
            "graph",
            "neighbors",
            file.to_str().ok_or("UTF-8 fixture path required")?,
            "-c",
            "Notes",
            "--relation",
            relation,
            "--depth",
            "255",
        ],
    )?;
    assert!(outgoing.status.success(), "{outgoing:?}");
    let text = String::from_utf8(outgoing.stdout)?;
    assert!(text.contains(&format!("{relation} {kind} {key}")), "{text}");
    assert!(text.contains("depth 1"));
    assert!(!text.contains(if relation == "ALIAS_OF" {
        "TAGGED_WITH"
    } else {
        "ALIAS_OF"
    }));
    Ok(())
}

/// Covers: REQ-023 FR-010/015 — enrichment preserves either mode's ranked results.
#[rstest]
#[case("lexical")]
#[case("hybrid")]
fn related_preserves_ranked_results_in_both_modes(
    #[case] mode: &str,
) -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"alpha [b](b.md)")?;
    SqliteFileStore::open_for_ingestion(&database)?.reconcile(
        &CollectionName::try_from("Notes")?,
        &[FileRecord::new(dir.path().join("b.md"), b"alpha".to_vec())],
        &[],
        Timestamp::from_unix_seconds(43),
    )?;
    let search = |related: bool| -> Result<Output, std::io::Error> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mdsearch"));
        command
            .env_remove("HOME")
            .env_remove("HF_HOME")
            .env("FASTEMBED_CACHE_DIR", dir.path().join("cache"))
            .arg("--database")
            .arg(&database)
            .args(["search", "alpha", "--mode", mode, "-c", "Notes", "--json"]);
        if mode == "hybrid" {
            command.arg("--no-rerank");
        }
        if related {
            command.arg("--related");
        }
        command.output()
    };
    let plain = search(false)?;
    let enriched = search(true)?;
    assert!(plain.status.success(), "{plain:?}");
    assert!(enriched.status.success(), "{enriched:?}");
    let expected: serde_json::Value = serde_json::from_slice(&plain.stdout)?;
    let mut actual: serde_json::Value = serde_json::from_slice(&enriched.stdout)?;
    let results = actual
        .get_mut("results")
        .ok_or("results required")?
        .as_array_mut()
        .ok_or("results array required")?;
    assert!(results.iter().any(|entry| {
        entry
            .get("related")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|related| !related.is_empty())
    }));
    for entry in results {
        entry
            .as_object_mut()
            .ok_or("result object required")?
            .remove("related");
    }
    assert_eq!(actual, expected);
    Ok(())
}

fn json_field<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
) -> Result<&'a serde_json::Value, Box<dyn Error>> {
    value
        .pointer(pointer)
        .ok_or_else(|| format!("missing JSON field: {pointer}").into())
}

fn create_and_update_two_sources(
    database: &Path,
    good: &Path,
    bad: &Path,
) -> Result<(), Box<dyn Error>> {
    for (name, source) in [("Notes", &good), ("Archive", &bad)] {
        let create = invoke(
            database,
            &[
                "collection",
                "create",
                name,
                source.to_str().ok_or("UTF-8 fixture path required")?,
            ],
        )?;
        assert!(create.status.success(), "{create:?}");
    }
    let initial = invoke(database, &["update", "--all"])?;
    assert!(initial.status.success(), "{initial:?}");
    Ok(())
}

fn assert_current_indexes(value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        json_field(value, "/collections/0/indexes/lexical/freshness")?,
        "current"
    );
    assert_eq!(
        json_field(value, "/collections/0/indexes/graph/freshness")?,
        "current"
    );
    assert_eq!(
        json_field(value, "/collections/0/indexes/semantic/enabled")?,
        false
    );
    Ok(())
}

fn seed_two_graphs(database: &Path, a: &Path, b: &Path, c: &Path) -> Result<(), Box<dyn Error>> {
    let at = Timestamp::from_unix_seconds(42);
    seed(database, a, b"[b](b.md)")?;
    let mut files = SqliteFileStore::open_for_ingestion(database)?;
    let notes = CollectionName::try_from("Notes")?;
    files.reconcile(
        &notes,
        &[
            FileRecord::new(b.to_owned(), b"[c](c.md)".to_vec()),
            FileRecord::new(c.to_owned(), b"last".to_vec()),
        ],
        &[],
        at,
    )?;
    let archive = CollectionName::try_from("Archive")?;
    SqliteCollectionStore::open(database)?.create_collection(&archive, at)?;
    files.reconcile(
        &archive,
        &[FileRecord::new(a.to_owned(), b"archive only".to_vec())],
        &[],
        at,
    )?;
    Ok(())
}

fn assert_stale_indexes(value: &serde_json::Value) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        json_field(value, "/collections/0/indexes/lexical/freshness")?,
        "stale"
    );
    assert_eq!(
        json_field(value, "/collections/0/indexes/graph/freshness")?,
        "stale"
    );
    assert_eq!(
        json_field(value, "/collections/0/indexes/lexical/built_at")?,
        42
    );
    assert_eq!(
        json_field(value, "/collections/0/indexes/graph/built_at")?,
        42
    );
    assert!(json_field(value, "/models/embedding/name")?.is_string());
    assert!(json_field(value, "/models/reranker/name")?.is_string());
    Ok(())
}

/// Covers: REQ-023 FR-022 — scoped status retains only the selected collection.
#[test]
fn status_scope_selects_one_of_multiple_collections() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed_two_graphs(
        &database,
        &dir.path().join("a.md"),
        &dir.path().join("b.md"),
        &dir.path().join("c.md"),
    )?;
    let output = invoke(&database, &["status", "-c", "Archive", "--json"])?;
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(
        json_field(&value, "/collections")?
            .as_array()
            .ok_or("collections array required")?
            .len(),
        1
    );
    assert_eq!(json_field(&value, "/collections/0/name")?, "Archive");
    Ok(())
}

/// Covers: REQ-023 FR-024 — lexical-only hybrid retrieval needs no model assets.
#[test]
fn lexical_only_hybrid_without_reranking_does_not_resolve_home() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    seed(&database, &dir.path().join("a.md"), b"alpha")?;
    let output = invoke(
        &database,
        &[
            "search",
            "alpha",
            "--mode",
            "hybrid",
            "--no-rerank",
            "--json",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json_field(&value, "/mode")?, "hybrid");
    assert_eq!(json_field(&value, "/results/0/text")?, "alpha");
    Ok(())
}

/// Covers: REQ-023 FR-004 — deleting stored indexes preserves registered original files.
#[test]
fn collection_delete_retains_original_source_bytes() -> Result<(), Box<dyn Error>> {
    let dir = tempdir()?;
    let database = dir.path().join("db");
    let file = dir.path().join("a.md");
    fs::write(&file, b"original")?;
    let create = invoke(
        &database,
        &[
            "collection",
            "create",
            "Notes",
            file.to_str().ok_or("UTF-8 fixture path required")?,
        ],
    )?;
    assert!(create.status.success(), "{create:?}");
    let update = invoke(&database, &["update", "-c", "Notes"])?;
    assert!(update.status.success(), "{update:?}");
    let deleted = invoke(&database, &["collection", "delete", "Notes"])?;
    assert!(deleted.status.success(), "{deleted:?}");
    assert_eq!(fs::read(file)?, b"original");
    let status = invoke(&database, &["status", "--json"])?;
    let value: serde_json::Value = serde_json::from_slice(&status.stdout)?;
    assert_eq!(json_field(&value, "/collections")?, &serde_json::json!([]));
    Ok(())
}
