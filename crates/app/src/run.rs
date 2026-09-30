use std::env;
use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kv_application::{
    AddFiles, Clock, CollectionSourceStore, CreateCollection, DestroyCollection, EmbedCollections,
    EmbedScope, EmbeddingGenerator, FileSystem, GetFile, GraphStore, HybridResult, HybridSearch,
    ListCollections, ReadIndexStatus, Reranker, SearchLexical, SearchResult, SearchScope,
    SemanticIndexStore, SemanticUpdateCoordinator, SemanticUpdatePorts, UpdateOutcome,
};
use kv_domain::{CollectionName, EmbeddingModel, EntityKind, NodeId, RerankerModel};
use kv_embed_fastembed::{FastembedGenerator, FastembedReranker};
use kv_infrastructure::{SystemClock, SystemFileSystem};
use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileRetrievalStore, SqliteFileStore, SqliteGraphStore,
    SqliteHybridSearchStore, SqliteLexicalIndexStore, SqliteLexicalSearchStore,
    SqliteSemanticIndexStore,
};

use crate::AppError;
use crate::cli::{
    Cli, CollectionCommand, Command, GraphCommand, HybridArgs, IndexCommand, ModelCommand,
    SearchArgs,
};
use crate::graph_query::{build_schema, handle};
use crate::model_cache;
use crate::progress;
use crate::related::{RelatedFile, related_files};
use crate::rendering::{
    render_embed_report, render_human, render_hybrid_human, render_hybrid_json,
    render_index_status, render_json,
};

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
pub fn run<I, T>(args: I, home_directory: &Path) -> Result<String, AppError>
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
pub fn run_from_environment<I, T>(args: I) -> Result<String, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;
    let uses_default_database = cli.database.is_none();
    let model_cache_environment = model_cache::ModelCacheEnvironment::from_process();
    let uses_default_model_cache = matches!(
        &cli.command,
        Command::Embed(_)
            | Command::Hybrid(_)
            | Command::Model(_)
            | Command::Collection(CollectionCommand::Update(_))
    ) && model_cache_environment.needs_home();
    let home_directory = if uses_default_database || uses_default_model_cache {
        Some(
            env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or(AppError::HomeUnavailable)?,
        )
    } else {
        None
    };

    run_cli(cli, home_directory.as_deref(), &model_cache_environment)
}

fn run_cli(
    cli: Cli,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let database = cli.database;

    match cli.command {
        Command::Collection(command) => {
            collection_command(command, database, home_directory, model_cache_environment)
        }
        Command::Index(IndexCommand::Status(_)) => index_status(database, home_directory),
        Command::Search(arguments) => search(&arguments, database, home_directory),
        Command::Get(arguments) => get_file(
            &arguments.collection,
            &arguments.name_or_id,
            database,
            home_directory,
        ),
        Command::Embed(arguments) => embed(
            arguments.collection.as_deref(),
            arguments.model.as_deref(),
            arguments.reranker.as_deref(),
            arguments.download,
            database.clone(),
            home_directory,
            model_cache_environment,
        ),
        Command::Model(ModelCommand::List(arguments)) => model_list(
            arguments.json,
            database.clone(),
            home_directory,
            model_cache_environment,
        ),
        Command::Model(ModelCommand::Set(arguments)) => model_set(
            arguments.model.as_deref(),
            arguments.reranker.as_deref(),
            arguments.download,
            database.clone(),
            home_directory,
            model_cache_environment,
        ),
        Command::Hybrid(arguments) => hybrid(
            &arguments,
            database.clone(),
            home_directory,
            model_cache_environment,
        ),
        Command::Graph(GraphCommand::Neighbors(arguments)) => graph_neighbors(
            &arguments.node,
            arguments.collection.as_deref(),
            database.clone(),
            home_directory,
        ),
        Command::Context(arguments) => context(
            &arguments.query,
            &arguments.collection,
            database,
            home_directory,
        ),
    }
}

fn collection_command(
    command: CollectionCommand,
    database: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    match command {
        CollectionCommand::Create(arguments) => create_collection(
            &arguments.name,
            &arguments.paths,
            arguments.semantic,
            database,
            home_directory,
        ),
        CollectionCommand::List(arguments) => {
            list_collections(arguments.json, database, home_directory)
        }
        CollectionCommand::Configure(arguments) => configure_collection(
            &arguments.name,
            &arguments.paths,
            arguments.semantic.as_deref(),
            database,
            home_directory,
        ),
        CollectionCommand::Destroy(arguments) => {
            destroy_collection(&arguments.name, database, home_directory)
        }
        CollectionCommand::Add(arguments) => add_files(
            &arguments.name,
            &arguments.paths,
            database,
            arguments.skip_unreadable,
            home_directory,
        ),
        CollectionCommand::Update(arguments) => update_command(
            &arguments,
            database,
            home_directory,
            model_cache_environment,
        ),
    }
}

fn update_command(
    arguments: &crate::cli::UpdateCollectionArgs,
    database: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    if arguments.all {
        return update_all_collections(
            database,
            arguments.skip_unreadable,
            arguments.download,
            home_directory,
            model_cache_environment,
        );
    }
    let name = arguments
        .name
        .as_deref()
        .or(arguments.legacy_name.as_deref())
        .ok_or_else(|| {
            AppError::Arguments(clap::Error::new(
                clap::error::ErrorKind::MissingRequiredArgument,
            ))
        })?;
    update_collection(
        name,
        database,
        arguments.skip_unreadable,
        home_directory,
        arguments.download,
        model_cache_environment,
    )
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
    migrate_existing_database(&database_path)?;
    let store = SqliteCollectionStore::open_existing(&database_path)?;
    let collections = store.list_collections_with_sources()?;
    if json {
        let value = serde_json::json!({"collections": collections.iter().map(|entry| serde_json::json!({
            "name": entry.name.display_name(),
            "sources": entry.sources.iter().map(|source| source.path().to_string_lossy()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()});
        return serde_json::to_string(&value)
            .map_err(|error| AppError::GraphQuery(error.to_string()));
    }
    Ok(collections
        .iter()
        .map(|entry| {
            if entry.sources.is_empty() {
                entry.name.display_name().to_owned()
            } else {
                format!(
                    "{}\n  {}",
                    entry.name.display_name(),
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

fn add_files(
    raw_name: &str,
    paths: &[PathBuf],
    database_override: Option<PathBuf>,
    force: bool,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let mut use_case = AddFiles::new(SystemFileSystem, store, SystemClock);
    let outcome = use_case.execute(&name, paths, force)?;

    let file_label = if outcome.added() == 1 {
        "file"
    } else {
        "files"
    };
    let mut message = format!(
        "added {} {} to collection \"{}\"",
        outcome.added(),
        file_label,
        name.display_name()
    );
    if outcome.skipped() > 0 {
        // Writing to a `String` cannot fail.
        let _ = write!(message, " (skipped {})", outcome.skipped());
    }

    Ok(message)
}

fn update_collection(
    raw_name: &str,
    database_override: Option<PathBuf>,
    force: bool,
    home_directory: Option<&Path>,
    download: bool,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_path(database_override, home_directory)?;
    let sources_store = SqliteCollectionStore::open_existing(&database_path)?;
    let sources = sources_store.collection_sources(&name)?;
    if sources.is_empty() {
        return Err(AppError::NoRegisteredSources(
            name.display_name().to_owned(),
        ));
    }
    let files = expand_sources(&sources)?;
    let store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let ports = SemanticUpdatePorts::new(
        SystemFileSystem,
        store,
        FastembedGenerator::new(cache_dir),
        SystemClock,
    );
    let mut coordinator = SemanticUpdateCoordinator::new(ports);
    let outcome = coordinator.execute(&name, &files, force, download)?;

    Ok(format_update(name.display_name(), &outcome))
}

fn update_all_collections(
    database_override: Option<PathBuf>,
    force: bool,
    download: bool,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    migrate_existing_database(&database_path)?;
    let collection_store = SqliteCollectionStore::open_existing(&database_path)?;
    let summaries = collection_store.list_collections_with_sources()?;
    drop(collection_store);
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    for summary in &summaries {
        let result = if summary.sources.is_empty() {
            Err(AppError::NoRegisteredSources(
                summary.name.display_name().to_owned(),
            ))
        } else {
            expand_sources(&summary.sources).and_then(|files| {
                let store = SqliteFileStore::open_for_ingestion(&database_path)?;
                let cache_dir =
                    model_cache::model_cache_dir(home_directory, model_cache_environment)
                        .ok_or(AppError::HomeUnavailable)?;
                let ports = SemanticUpdatePorts::new(
                    SystemFileSystem,
                    store,
                    FastembedGenerator::new(cache_dir),
                    SystemClock,
                );
                let mut coordinator = SemanticUpdateCoordinator::new(ports);
                let outcome = coordinator.execute(&summary.name, &files, force, download)?;
                Ok(format_update(summary.name.display_name(), &outcome))
            })
        };
        match result {
            Ok(line) => lines.push(line),
            Err(error) => {
                let line = format!(
                    "update failed for collection \"{}\": {error}",
                    summary.name.display_name()
                );
                lines.push(line.clone());
                failures.push(line);
            }
        }
    }
    if failures.is_empty() {
        Ok(lines.join("\n"))
    } else {
        Err(AppError::UpdateAllFailed(lines.join("\n")))
    }
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
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteLexicalIndexStore::open(&database_path)?;
    let use_case = ReadIndexStatus::new(store);
    let statuses = use_case.execute()?;

    Ok(statuses
        .iter()
        .map(render_index_status)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn search(
    args: &SearchArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    if args.query.trim().is_empty() {
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

    let set = use_case.execute(&args.query, usize::from(args.limit), scope)?;

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
            &args.query,
            &scope_name,
            args.limit,
            related_context.as_deref(),
        ))
    } else {
        Ok(render_human(&set, related_context.as_deref()))
    }
}

fn hybrid(
    args: &HybridArgs,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    if args.query.trim().is_empty() {
        return Err(AppError::Hybrid(kv_application::HybridError::EmptyQuery));
    }

    let database_path = database_path(database_override, home_directory)?;
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir);
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

    let set = use_case.execute(&args.query, usize::from(args.limit), scope, !args.no_rerank)?;

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
            &args.query,
            &scope_name,
            args.limit,
            related_context.as_deref(),
        )
    } else {
        render_hybrid_human(&set, related_context.as_deref())
    };

    Ok(rendered)
}

fn get_file(
    raw_collection: &str,
    name_or_id: &str,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let collection = CollectionName::try_from(raw_collection)?;
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteFileRetrievalStore::open(&database_path)?;
    let use_case = GetFile::new(store);
    let file = use_case.execute(&collection, name_or_id)?;

    String::from_utf8(file.content().to_vec()).map_err(|_| AppError::NonUtf8Content)
}

fn graph_neighbors(
    raw_node: &str,
    collection_name: Option<&str>,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteGraphStore::open(&database_path)?;

    let collections: Vec<CollectionName> = if let Some(name) = collection_name {
        vec![CollectionName::try_from(name)?]
    } else {
        let collection_store = SqliteCollectionStore::open_existing(&database_path)?;
        ListCollections::new(collection_store).execute()?
    };

    let mut lines = Vec::new();
    for collection in &collections {
        for kind in [EntityKind::File, EntityKind::Tag, EntityKind::Alias] {
            let id = NodeId::new(kind, raw_node.to_owned());
            if store.node(collection, &id)?.is_none() {
                continue;
            }
            let neighbors = store.neighbors(collection, &id, None, 3)?;
            lines.push(format!("{}:", id.key()));
            for neighbor in neighbors {
                lines.push(format!(
                    "  {} {} (depth {})",
                    neighbor.relation().as_str(),
                    neighbor.node().id().key(),
                    neighbor.depth()
                ));
            }
            return Ok(lines.join("\n"));
        }
    }

    Err(AppError::Graph(kv_application::GraphStoreError::Storage(
        format!("node not found: {raw_node}").into(),
    )))
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
    let _collection = CollectionName::try_from(collection_name)?;
    let database_path = database_path(database_override, home_directory)?;
    let store = SqliteGraphStore::open(&database_path)?;
    let schema = build_schema(handle(store));

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

fn embed(
    collection_name: Option<&str>,
    model_name: Option<&str>,
    reranker_name: Option<&str>,
    download: bool,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let database_path = database_path(database_override, home_directory)?;
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir);
    let store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let mut use_case = EmbedCollections::new(generator, store, SystemClock, reranker);

    let model = model_name.map(EmbeddingModel::try_from).transpose()?;
    let reranker = reranker_name.map(RerankerModel::try_from).transpose()?;
    let collection = collection_name.map(CollectionName::try_from).transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => EmbedScope::Collection(collection),
        None => EmbedScope::All,
    };

    let mut progress_view = progress::ProgressRenderer::default();
    let report = use_case.execute(
        scope,
        model.as_ref(),
        reranker.as_ref(),
        download,
        &mut |event| {
            progress_view.handle(event);
        },
    )?;
    progress_view.finish();

    let rendered = render_embed_report(&report);
    if report.any_failed() {
        Err(AppError::EmbedPartial(rendered))
    } else {
        Ok(rendered)
    }
}

fn model_list(
    json: bool,
    _database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir);
    let embeddings = generator.models();
    let rerankers = reranker.models();

    if json {
        let value = serde_json::json!({
            "embedding": embeddings.iter().map(|model| serde_json::json!({
                "name": model.name(), "available": model.available()
            })).collect::<Vec<_>>(),
            "reranker": rerankers.iter().map(|model| serde_json::json!({
                "name": model.name(), "available": model.available()
            })).collect::<Vec<_>>(),
        });
        return serde_json::to_string(&value)
            .map_err(|error| AppError::GraphQuery(error.to_string()));
    }

    let mut lines = embeddings
        .iter()
        .map(|model| {
            format!(
                "embedding {}: {}",
                model.name(),
                model_state(model.available())
            )
        })
        .collect::<Vec<_>>();
    lines.extend(rerankers.iter().map(|model| {
        format!(
            "reranker {}: {}",
            model.name(),
            model_state(model.available())
        )
    }));
    Ok(lines.join("\n"))
}

fn model_state(available: bool) -> &'static str {
    if available {
        "available"
    } else {
        "not downloaded"
    }
}

fn model_set(
    model_name: Option<&str>,
    reranker_name: Option<&str>,
    download: bool,
    database_override: Option<PathBuf>,
    home_directory: Option<&Path>,
    model_cache_environment: &model_cache::ModelCacheEnvironment,
) -> Result<String, AppError> {
    if model_name.is_none() && reranker_name.is_none() {
        return Err(AppError::Arguments(clap::Error::new(
            clap::error::ErrorKind::MissingRequiredArgument,
        )));
    }
    let cache_dir = model_cache::model_cache_dir(home_directory, model_cache_environment)
        .ok_or(AppError::HomeUnavailable)?;
    let generator = FastembedGenerator::new(cache_dir.clone());
    let reranker = FastembedReranker::new(cache_dir);
    if let Some(raw_model) = model_name {
        let model = EmbeddingModel::try_from(raw_model)?;
        generator
            .ensure_available(&model, download)
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    if let Some(raw_reranker) = reranker_name {
        let reranker_model = RerankerModel::try_from(raw_reranker)?;
        reranker
            .ensure_available(&reranker_model, download)
            .map_err(|error| AppError::Model(error.to_string()))?;
    }
    embed(
        None,
        model_name,
        reranker_name,
        download,
        database_override,
        home_directory,
        model_cache_environment,
    )
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

fn database_path(
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
            collection("Notes")?,
            PathBuf::from(path),
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
