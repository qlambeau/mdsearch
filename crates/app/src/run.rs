use std::env;
use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kv_application::{
    Clock, CollectionSourceStore, CreateCollection, DestroyCollection, FileSystem, GetFile,
    GraphStore, HybridResult, HybridSearch, ListCollections, ReadIndexStatus, SearchLexical,
    SearchResult, SearchScope, SemanticIndexStore, SemanticUpdateCoordinator, SemanticUpdatePorts,
    UpdateOutcome,
};
use kv_domain::{CollectionName, EntityKind, NodeId};
use kv_infrastructure::{SystemClock, SystemFileSystem};
use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileRetrievalStore, SqliteFileStore, SqliteGraphStore,
    SqliteHybridSearchStore, SqliteIndexInspectionStore, SqliteLexicalSearchStore,
    SqliteSemanticIndexStore,
};

use crate::cli::{
    Cli, CollectionCommand, Command, GraphCommand, ModelCommand, SearchArgs, SearchMode,
};
use crate::graph_query::build_schema;
use crate::model_cache;
use crate::models::{model_list, model_set};
use crate::related::{RelatedFile, related_files};
use crate::rendering::{render_human, render_hybrid_human, render_hybrid_json, render_json};
use crate::{AppError, CommandOutput};

/// Executes one `mdsearch` CLI invocation with an injected home directory.
///
/// # Errors
///
/// Returns an argument, name-validation, database, or application error when
/// the invocation cannot complete.
///
/// # Examples
///
/// ```no_run
/// let output = kv_app::run(["mdsearch", "--version"], std::path::Path::new("."))?;
/// # Ok::<(), kv_app::AppError>(())
/// ```
pub fn run<I, T>(args: I, home_directory: &Path) -> Result<CommandOutput, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    let model_cache_environment = model_cache::ModelCacheEnvironment::from_process();
    run_cli(cli, Some(home_directory), &model_cache_environment)
}

/// Executes a CLI invocation using paths from the process environment.
///
/// # Errors
///
/// Returns an argument or operational error when the command cannot complete.
///
/// # Examples
///
/// ```no_run
/// let output = kv_app::run_from_environment(std::env::args_os())?;
/// # Ok::<(), kv_app::AppError>(())
/// ```
pub fn run_from_environment<I, T>(args: I) -> Result<CommandOutput, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    let model_cache_environment = model_cache::ModelCacheEnvironment::from_process();
    let home_directory = env::var_os("HOME").map(PathBuf::from);

    run_cli(cli, home_directory.as_deref(), &model_cache_environment)
}

fn run_cli(
    cli: Cli,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<CommandOutput, AppError> {
    let database = cli.database;
    let result = match cli.command {
        Command::Collection(command) => collection_command(command, database, home_directory),
        Command::Update(arguments) => update_command(
            &arguments,
            database,
            home_directory,
            model_cache_environment,
        ),
        Command::Status(arguments) => index_status(&arguments, database, home_directory),
        Command::Search(arguments) => {
            if arguments.no_rerank && arguments.mode == SearchMode::Lexical {
                return Err(AppError::Arguments(clap::Error::raw(
                    clap::error::ErrorKind::ArgumentConflict,
                    "--no-rerank requires --mode hybrid",
                )));
            }
            match arguments.mode {
                SearchMode::Lexical => search(&arguments, database, home_directory),
                SearchMode::Hybrid => hybrid(
                    &arguments,
                    database,
                    home_directory,
                    model_cache_environment,
                ),
            }
        }
        Command::Get(arguments) => {
            let collection = CollectionName::try_from(arguments.collection.as_str())?;
            let path = database_path(database, home_directory)?;
            let use_case = GetFile::new(SqliteFileRetrievalStore::open(&path)?);
            let selector = match arguments.id {
                Some(id) => kv_application::FileSelector::Id(
                    kv_domain::FileId::try_new(id)
                        .map_err(|error| AppError::Model(error.to_string()))?,
                ),
                None => kv_application::FileSelector::Name(
                    arguments.path_or_name.as_deref().ok_or_else(|| {
                        AppError::Arguments(clap::Error::new(
                            clap::error::ErrorKind::MissingRequiredArgument,
                        ))
                    })?,
                ),
            };
            return Ok(CommandOutput::Content(
                use_case.execute(&collection, selector)?.content().to_vec(),
            ));
        }
        Command::Model(ModelCommand::List(arguments)) => model_list(
            arguments.json,
            database,
            home_directory,
            model_cache_environment,
        ),
        Command::Model(ModelCommand::Set(arguments)) => model_set(
            &arguments,
            database,
            home_directory,
            model_cache_environment,
        ),
        Command::Graph(GraphCommand::Neighbors(arguments)) => {
            graph_neighbors(&arguments, database, home_directory)
        }
        Command::Graph(GraphCommand::Query(arguments)) => context(
            &arguments.query,
            &arguments.collection,
            database,
            home_directory,
        ),
    };
    result.map(CommandOutput::Report)
}

fn collection_command(
    command: CollectionCommand,
    database: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    match command {
        CollectionCommand::Create(args) => create_collection(
            &args.name,
            &args.paths,
            args.semantic,
            database,
            home_directory,
        ),
        CollectionCommand::List(args) => list_collections(args.json, database, home_directory),
        CollectionCommand::Configure(args) => configure_collection(
            &args.name,
            &args.paths,
            args.semantic.as_deref(),
            database,
            home_directory,
        ),
        CollectionCommand::Delete(args) => destroy_collection(&args.name, database, home_directory),
    }
}

fn update_command(
    args: &crate::cli::UpdateCollectionArgs,
    database: Option<PathBuf>,
    home_directory: Option<&Path>,
    environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let path = database_path(database, home_directory)?;
    migrate_existing_database(&path)?;
    let store = SqliteCollectionStore::open_existing(&path)?;
    let names = match args.scope.name.as_deref() {
        Some(name) => vec![CollectionName::try_from(name)?],
        None => ListCollections::new(store).execute()?,
    };
    let mut failed = false;
    let mut entries = Vec::new();
    let mut lines = Vec::new();
    for name in names {
        match update_collection(&name, args, &path, home_directory, environment) {
            Ok(outcome) => {
                lines.push(format_update(name.display_name(), &outcome));
                entries.push(serde_json::json!({"name":name.display_name(), "success":true, "added":outcome.added(), "modified":outcome.modified(), "deleted":outcome.deleted(), "skipped":outcome.skipped(), "malformed_frontmatter":outcome.malformed_frontmatter()}));
            }
            Err(error) => {
                failed = true;
                lines.push(format!(
                    "update failed for collection \"{}\": {error}",
                    name.display_name()
                ));
                entries.push(serde_json::json!({"name":name.display_name(), "success":false, "diagnostic":error.to_string()}));
            }
        }
    }
    let report = if args.json {
        serde_json::json!({"collections":entries}).to_string()
    } else {
        lines.join("\n")
    };
    if failed {
        Err(AppError::UpdateReportFailed(report))
    } else {
        Ok(report)
    }
}

fn create_collection(
    raw_name: &str,
    paths: &[PathBuf],
    semantic: bool,
    database_override: Option<std::path::PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_path(database_override, home_directory)?;
    let mut store = SqliteCollectionStore::open(&database_path)?;
    let sources = resolve_sources(paths)?;
    let created_name = if sources.is_empty() {
        CreateCollection::new(store, SystemClock).execute(name)?
    } else {
        let created_at = SystemClock.now()?;
        store.create_collection_with_sources(&name, created_at, &sources)?;
        name
    };

    if semantic {
        let mut semantic_store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
        semantic_store.set_semantic_enabled(&created_name, true)?;
    }

    Ok(format!(
        "created collection \"{}\"",
        created_name.display_name()
    ))
}

fn list_collections(
    json: bool,
    database_override: Option<std::path::PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteCollectionStore::open_existing(&database_path)?;
    let collections = store.list_collections_with_sources()?;
    if json {
        let value = serde_json::json!({"collections": collections.iter().map(|entry| serde_json::json!({
            "name": entry.name.display_name(),
            "indexes": {"lexical":true, "graph":true, "semantic":entry.semantic_enabled},
            "sources": entry.sources.iter().map(|source| source.path().to_string_lossy()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()});
        return serde_json::to_string(&value)
            .map_err(|error| AppError::GraphQuery(error.to_string()));
    }
    Ok(collections
        .iter()
        .map(|entry| {
            if entry.sources.is_empty() {
                format!(
                    "{} [lexical, graph{}]",
                    entry.name.display_name(),
                    if entry.semantic_enabled {
                        ", semantic"
                    } else {
                        ""
                    }
                )
            } else {
                format!(
                    "{} [lexical, graph{}]\n  {}",
                    entry.name.display_name(),
                    if entry.semantic_enabled {
                        ", semantic"
                    } else {
                        ""
                    },
                    entry
                        .sources
                        .iter()
                        .map(|source| source.path().display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n  ")
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn configure_collection(
    raw_name: &str,
    paths: &[PathBuf],
    semantic: Option<&str>,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_path(database_override, home_directory)?;
    migrate_existing_database(&database_path)?;
    if paths.is_empty() {
        let store = SqliteCollectionStore::open_existing(&database_path)?;
        store.collection_sources(&name)?;
    } else {
        let mut store = SqliteCollectionStore::open_existing(&database_path)?;
        let sources = resolve_sources(paths)?;
        store.replace_collection_sources(&name, &sources)?;
    }
    if let Some(value) = semantic {
        let mut store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
        store.set_semantic_enabled(&name, value == "on")?;
    }
    Ok(format!(
        "configured sources for collection \"{}\"",
        name.display_name()
    ))
}

fn resolve_sources(paths: &[PathBuf]) -> Result<Vec<kv_domain::CollectionSource>, AppError> {
    let mut sources = Vec::new();
    let mut unique = std::collections::BTreeSet::new();
    for path in paths {
        let source = kv_infrastructure::SystemFileSystem.resolve_source(path)?;
        if unique.insert(source.path().to_owned()) {
            sources.push(source);
        }
    }
    Ok(sources)
}

fn destroy_collection(
    raw_name: &str,
    database_override: Option<std::path::PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_path(database_override, home_directory)?;
    migrate_existing_database(&database_path)?;
    let store = SqliteCollectionStore::open_existing(&database_path)?;
    let mut use_case = DestroyCollection::new(store);
    let destroyed_name = use_case.execute(&name)?;

    Ok(format!(
        "destroyed collection \"{}\"",
        destroyed_name.display_name()
    ))
}

fn update_collection(
    name: &CollectionName,
    args: &crate::cli::UpdateCollectionArgs,
    database_path: &Path,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<UpdateOutcome, AppError> {
    let sources_store = SqliteCollectionStore::open_existing(database_path)?;
    let sources = sources_store.collection_sources(name)?;
    if sources.is_empty() {
        return Err(AppError::NoRegisteredSources(
            name.display_name().to_owned(),
        ));
    }
    let files = expand_sources(&sources)?;
    let store = SqliteFileStore::open_for_ingestion(database_path)?;
    let generator = crate::model_assets::ModelGenerator::new(model_cache::model_cache_dir(
        home_directory,
        model_cache_environment,
    ));
    let ports = SemanticUpdatePorts::new(SystemFileSystem, store, generator, SystemClock);
    let mut coordinator = SemanticUpdateCoordinator::new(ports);
    let outcome = coordinator.execute(name, &files, args.skip_unreadable, args.download)?;

    Ok(outcome)
}

fn expand_sources(sources: &[kv_domain::CollectionSource]) -> Result<Vec<PathBuf>, AppError> {
    use kv_domain::SourceKind;
    let filesystem = SystemFileSystem;
    let mut files = std::collections::BTreeSet::new();
    for source in sources {
        if source.kind() == SourceKind::File && !filesystem.exists(source.path())? {
            continue;
        }
        for path in filesystem.expand(source.path())? {
            files.insert(path);
        }
    }
    Ok(files.into_iter().collect())
}

fn migrate_existing_database(database_path: &Path) -> Result<(), AppError> {
    drop(SqliteFileStore::open_for_ingestion(database_path)?);
    Ok(())
}

fn index_status(
    args: &crate::cli::IndexStatusArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let path = database_path(database_override, home_directory)?;
    let scope = args
        .collection
        .as_deref()
        .map(CollectionName::try_from)
        .transpose()?;
    let report =
        ReadIndexStatus::new(SqliteIndexInspectionStore::open(&path)?).inspect(scope.as_ref())?;
    Ok(crate::status_rendering::render(&report, args.json))
}

fn search(
    args: &SearchArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let query = args.query.join(" ");
    if query.trim().is_empty() {
        return Err(AppError::Search(kv_application::SearchError::EmptyQuery));
    }

    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteLexicalSearchStore::open(&database_path)?;
    let use_case = SearchLexical::new(store);

    let collection = args
        .collection
        .as_deref()
        .map(CollectionName::try_from)
        .transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => SearchScope::Collection(collection),
        None => SearchScope::All,
    };

    let set = use_case.execute(&query, usize::from(args.limit), scope)?;

    let related_context = if args.related {
        let graph_store = SqliteGraphStore::open(&database_path)?;
        Some(collect_related(&graph_store, set.results()))
    } else {
        None
    };

    let scope_name = collection.as_ref().map_or_else(
        || "all".to_owned(),
        |collection| collection.display_name().to_owned(),
    );

    if args.json {
        Ok(render_json(
            &set,
            &query,
            &scope_name,
            args.limit,
            related_context.as_deref(),
        ))
    } else {
        Ok(render_human(&set, related_context.as_deref()))
    }
}

fn hybrid(
    args: &SearchArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let query = args.query.join(" ");
    if query.trim().is_empty() {
        return Err(AppError::Hybrid(kv_application::HybridError::EmptyQuery));
    }

    let database_path = database_path(database_override, home_directory)?;
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment);
    let generator = crate::model_assets::ModelGenerator::new(cache_dir.clone());
    let reranker = crate::model_assets::ModelReranker::new(cache_dir);
    let store = SqliteHybridSearchStore::open(&database_path)?;
    let use_case = HybridSearch::new(generator, store, reranker);

    let collection = args
        .collection
        .as_deref()
        .map(CollectionName::try_from)
        .transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => SearchScope::Collection(collection),
        None => SearchScope::All,
    };

    let set = use_case
        .execute(&query, usize::from(args.limit), scope, !args.no_rerank)
        .map_err(|error| crate::recovery::hybrid(error, &database_path, collection.as_ref()))?;

    let related_context = if args.related {
        let graph_store = SqliteGraphStore::open(&database_path)?;
        Some(collect_related(&graph_store, set.results()))
    } else {
        None
    };

    let scope_name = collection.as_ref().map_or_else(
        || "all".to_owned(),
        |collection| collection.display_name().to_owned(),
    );

    let rendered = if args.json {
        render_hybrid_json(
            &set,
            &query,
            &scope_name,
            args.limit,
            related_context.as_deref(),
        )
    } else {
        render_hybrid_human(&set, related_context.as_deref())
    };

    Ok(rendered)
}

fn graph_neighbors(
    args: &crate::cli::GraphNeighborsArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteGraphStore::open(&database_path)?;
    let collection = CollectionName::try_from(args.collection.as_str())?;
    let kind = EntityKind::from_key(&args.kind)
        .ok_or_else(|| AppError::GraphQuery("unknown node kind".to_owned()))?;
    let relation = args
        .relation
        .as_deref()
        .and_then(kv_domain::RelationKind::from_key);
    let id = NodeId::new(kind, args.node.clone());
    if store.node(&collection, &id)?.is_none() {
        return Err(AppError::Graph(kv_application::GraphStoreError::Storage(
            format!("node not found: {} {}", args.kind, args.node).into(),
        )));
    }
    let mut lines = vec![format!("{} {}:", kind.as_str(), args.node)];
    for neighbor in store.neighbors(&collection, &id, relation, args.depth)? {
        lines.push(format!(
            "  {} {} {} — {} (depth {})",
            neighbor.relation().as_str(),
            neighbor.node().id().kind().as_str(),
            neighbor.node().id().key(),
            neighbor.node().title(),
            neighbor.depth()
        ));
    }
    Ok(lines.join("\n"))
}

/// Executes an in-process GraphQL query against the entity graph and prints the
/// JSON result.
///
/// The command is read-only: it opens the database without initializing it, so
/// a missing database fails without creating a file.
fn context(
    query: &str,
    collection_name: &str,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let collection = CollectionName::try_from(collection_name)?;
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteGraphStore::open(&database_path)?;
    SqliteCollectionStore::open_existing(&database_path)?.collection_sources(&collection)?;
    let schema = build_schema(store, collection);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|error| AppError::GraphQuery(error.to_string()))?;
    let response = runtime.block_on(schema.execute(query));

    if !response.errors.is_empty() {
        let message = response.errors.first().map_or_else(
            || "graph query failed".to_owned(),
            |error| error.message.clone(),
        );
        return Err(AppError::GraphQuery(message));
    }

    serde_json::to_string(&response.data).map_err(|error| AppError::GraphQuery(error.to_string()))
}

/// A ranked result that exposes the file whose related context is recovered.
trait RelatedResult {
    /// Returns the collection the result belongs to.
    fn collection(&self) -> &CollectionName;
    /// Returns the result file path.
    fn path(&self) -> &Path;
}

impl RelatedResult for SearchResult {
    fn collection(&self) -> &CollectionName {
        SearchResult::collection(self)
    }

    fn path(&self) -> &Path {
        SearchResult::path(self)
    }
}

impl RelatedResult for HybridResult {
    fn collection(&self) -> &CollectionName {
        HybridResult::collection(self)
    }

    fn path(&self) -> &Path {
        HybridResult::path(self)
    }
}

/// Collects the per-result related context in result order.
fn collect_related<T>(store: &dyn GraphStore, results: &[T]) -> Vec<Vec<RelatedFile>>
where
    T: RelatedResult,
{
    results
        .iter()
        .map(|result| related_files(store, result.collection(), result.path()))
        .collect()
}

fn format_update(display_name: &str, outcome: &UpdateOutcome) -> String {
    let mut line = format!(
        "updated collection \"{display_name}\": added {}, modified {}, deleted {}",
        outcome.added(),
        outcome.modified(),
        outcome.deleted()
    );
    if outcome.skipped() > 0 {
        // Writing to a `String` cannot fail.
        let _ = write!(line, " (skipped {})", outcome.skipped());
    }
    if outcome.malformed_frontmatter() > 0 {
        // Writing to a `String` cannot fail.
        let _ = write!(
            line,
            " ({} malformed frontmatter)",
            outcome.malformed_frontmatter()
        );
    }
    line
}

pub(super) fn database_path(
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<PathBuf, AppError> {
    if let Some(database_path) = database_override {
        return Ok(database_path);
    }

    let home_directory = home_directory.ok_or(AppError::HomeUnavailable)?;
    Ok(home_directory.join(".mdsearch").join("collections.db"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kv_application::{
        EmbedOutcome, EmbedReport, Position, SearchResult, SearchResultSet, SkipReason,
    };
    use kv_domain::{CollectionName, PassageKind, RelationKind};

    use crate::related::RelatedFile;

    use crate::rendering::{
        render_embed_outcome, render_embed_report, render_human, render_json, render_related_lines,
    };

    fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
        CollectionName::try_from(name)
    }

    fn result(path: &str) -> Result<SearchResult, Box<dyn std::error::Error>> {
        Ok(SearchResult::new(
            kv_application::SearchFile::new(
                kv_domain::FileId::try_new(1)?,
                collection("Notes")?,
                PathBuf::from(path),
            ),
            PassageKind::Body,
            "body text".to_owned(),
            1.0,
            Position::new(0, 10, 0, 0),
        ))
    }

    fn related(path: &str, relation: RelationKind) -> RelatedFile {
        RelatedFile::new(PathBuf::from(path), relation)
    }

    /// Covers: REQ-013 FR-002 — human output adds `related:` lines.
    #[test]
    fn human_output_adds_related_lines() -> Result<(), Box<dyn std::error::Error>> {
        let set = SearchResultSet::new(vec![result("a.md")?], 1);
        let related = vec![vec![related("b.md", RelationKind::LinksTo)]];
        let output = render_human(&set, Some(&related));
        assert!(output.contains("related: b.md (LINKS_TO)"));
        Ok(())
    }

    /// Covers: REQ-013 FR-002 — results without related files add no line.
    #[test]
    fn human_output_adds_no_line_without_related() -> Result<(), Box<dyn std::error::Error>> {
        let set = SearchResultSet::new(vec![result("a.md")?], 1);
        let output = render_human(&set, None);
        assert!(!output.contains("related:"));
        Ok(())
    }

    /// Covers: REQ-013 FR-003 — JSON output includes a related field.
    #[test]
    fn json_output_includes_related_field() -> Result<(), Box<dyn std::error::Error>> {
        let set = SearchResultSet::new(vec![result("a.md")?], 1);
        let related = vec![vec![related("b.md", RelationKind::LinksTo)]];
        let output = render_json(&set, "rust", "all", 10, Some(&related));
        let value: serde_json::Value = serde_json::from_str(&output)?;
        let entry = value
            .get("results")
            .and_then(|results| results.get(0))
            .and_then(|result| result.get("related"))
            .and_then(|related| related.get(0))
            .ok_or("expected a related entry")?;
        assert_eq!(entry["path"], "b.md");
        assert_eq!(entry["relation"], "LINKS_TO");
        Ok(())
    }

    /// Covers: REQ-013 FR-003 — JSON output omits the field without --related.
    #[test]
    fn json_output_omits_related_field_without_flag() -> Result<(), Box<dyn std::error::Error>> {
        let set = SearchResultSet::new(vec![result("a.md")?], 1);
        let output = render_json(&set, "rust", "all", 10, None);
        let value: serde_json::Value = serde_json::from_str(&output)?;
        let first = value
            .get("results")
            .and_then(|results| results.get(0))
            .ok_or("expected a result")?;
        assert!(first.get("related").is_none());
        Ok(())
    }

    /// Covers: REQ-013 FR-004 — ranked results are unchanged by --related.
    #[test]
    fn ranked_results_are_unchanged_by_related() -> Result<(), Box<dyn std::error::Error>> {
        let set = SearchResultSet::new(vec![result("a.md")?], 1);
        let related = vec![vec![related("b.md", RelationKind::LinksTo)]];
        let with = render_human(&set, Some(&related));
        let without = render_human(&set, None);
        let first_line_with = with.lines().next().ok_or("expected a header")?;
        let first_line_without = without.lines().next().ok_or("expected a header")?;
        assert_eq!(first_line_with, first_line_without);
        Ok(())
    }

    /// Covers: REQ-013 FR-002 — related rendering aligns by result index.
    #[test]
    fn related_lines_render_in_result_order() {
        let mut lines = Vec::new();
        let related = vec![vec![related("b.md", RelationKind::LinksTo)]];
        render_related_lines(&mut lines, Some(&related), 0);
        assert_eq!(lines, vec!["related: b.md (LINKS_TO)"]);
    }

    /// Covers: FR-016 — an embedded outcome reports its passage count.
    #[test]
    fn renders_an_embedded_outcome() -> Result<(), Box<dyn std::error::Error>> {
        let output = render_embed_outcome(&EmbedOutcome::Embedded {
            collection: collection("Notes")?,
            passage_count: 5,
        });

        assert_eq!(output, "collection \"Notes\": embedded 5 passage(s)");

        Ok(())
    }

    /// Covers: FR-016 — already-current, skipped, and failed outcomes render.
    #[test]
    fn renders_the_other_outcome_kinds() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            render_embed_outcome(&EmbedOutcome::AlreadyCurrent {
                collection: collection("Notes")?,
            }),
            "collection \"Notes\": already current"
        );
        assert_eq!(
            render_embed_outcome(&EmbedOutcome::Skipped {
                collection: collection("Empty")?,
                reason: SkipReason::NoFiles,
            }),
            "collection \"Empty\": skipped (no files)"
        );
        assert_eq!(
            render_embed_outcome(&EmbedOutcome::Skipped {
                collection: collection("Archive")?,
                reason: SkipReason::LexicalNotBuilt,
            }),
            "collection \"Archive\": skipped (lexical index not built)"
        );
        assert_eq!(
            render_embed_outcome(&EmbedOutcome::Failed {
                collection: collection("Notes")?,
                message: "embedding failed".to_owned(),
            }),
            "collection \"Notes\": failed (embedding failed)"
        );

        Ok(())
    }

    /// Covers: FR-015 — a report with failures adds a failure summary line.
    #[test]
    fn renders_a_failure_summary_for_a_partial_report() -> Result<(), Box<dyn std::error::Error>> {
        let mut report = EmbedReport::new();
        report.push(EmbedOutcome::Failed {
            collection: collection("Notes")?,
            message: "boom".to_owned(),
        });
        report.push(EmbedOutcome::Embedded {
            collection: collection("Archive")?,
            passage_count: 2,
        });

        let output = render_embed_report(&report);

        assert!(output.contains("collection \"Archive\": embedded 2 passage(s)"));
        assert!(output.contains("collection \"Notes\": failed (boom)"));
        assert!(output.contains("embedding completed with failures"));

        Ok(())
    }

    /// Covers: FR-015 — a fully successful report has no failure summary.
    #[test]
    fn renders_no_failure_summary_for_a_successful_report() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut report = EmbedReport::new();
        report.push(EmbedOutcome::Embedded {
            collection: collection("Notes")?,
            passage_count: 3,
        });

        let output = render_embed_report(&report);

        assert_eq!(output, "collection \"Notes\": embedded 3 passage(s)");

        Ok(())
    }
}
