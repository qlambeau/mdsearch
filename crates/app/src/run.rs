use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kv_application::{
    AddFiles, CreateCollection, DestroyCollection, EmbedCollections, EmbedOutcome, EmbedReport,
    EmbedScope, GetFile, GraphStore, HybridResult, HybridResultSet, HybridSearch, IndexState,
    IndexStatus, ListCollections, ReadIndexStatus, SearchLexical, SearchResult, SearchResultSet,
    SearchScope, SkipReason, UpdateCollection, UpdateOutcome, UpdateTarget,
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
    Cli, CollectionCommand, Command, GraphCommand, HybridArgs, IndexCommand, SearchArgs,
};
use crate::graph_query::{build_schema, handle};
use crate::related::{RelatedFile, related_files};

/// Executes one `mdsearch` CLI invocation with an injected home directory.
///
/// # Errors
///
/// Returns an argument, name-validation, database, or application error when
/// the invocation cannot complete.
pub fn run<I, T>(args: I, home_directory: &Path) -> Result<String, AppError>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(args)?;

    match cli.command {
        Command::Collection(CollectionCommand::Create(arguments)) => {
            create_collection(&arguments.name, arguments.database, home_directory)
        }
        Command::Collection(CollectionCommand::List(arguments)) => {
            list_collections(arguments.database, home_directory)
        }
        Command::Collection(CollectionCommand::Destroy(arguments)) => {
            destroy_collection(&arguments.name, arguments.database, home_directory)
        }
        Command::Collection(CollectionCommand::Add(arguments)) => add_files(
            &arguments.name,
            &arguments.paths,
            arguments.database,
            arguments.force,
            home_directory,
        ),
        Command::Collection(CollectionCommand::Update(arguments)) => {
            if arguments.all {
                update_all_collections(arguments.database, arguments.force, home_directory)
            } else {
                let name = arguments.name.as_deref().ok_or_else(|| {
                    AppError::Arguments(clap::Error::new(
                        clap::error::ErrorKind::MissingRequiredArgument,
                    ))
                })?;
                update_collection(
                    name,
                    &arguments.paths,
                    arguments.database,
                    arguments.force,
                    home_directory,
                )
            }
        }
        Command::Index(IndexCommand::Status(arguments)) => {
            index_status(arguments.database, home_directory)
        }
        Command::Search(arguments) => search(&arguments, home_directory),
        Command::Get(arguments) => get_file(
            &arguments.collection,
            &arguments.name_or_id,
            arguments.database,
            home_directory,
        ),
        Command::Embed(arguments) => embed(
            arguments.collection.as_deref(),
            arguments.model.as_deref(),
            arguments.reranker.as_deref(),
            arguments.download,
            arguments.database,
            home_directory,
        ),
        Command::Hybrid(arguments) => hybrid(&arguments, home_directory),
        Command::Graph(GraphCommand::Neighbors(arguments)) => graph_neighbors(
            &arguments.node,
            arguments.collection.as_deref(),
            arguments.database,
            home_directory,
        ),
        Command::Context(arguments) => context(
            &arguments.query,
            &arguments.collection,
            arguments.database,
            home_directory,
        ),
    }
}

fn create_collection(
    raw_name: &str,
    database_override: Option<std::path::PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteCollectionStore::open(&database_path)?;
    let mut use_case = CreateCollection::new(store, SystemClock);
    let created_name = use_case.execute(name)?;

    Ok(format!(
        "created collection \"{}\"",
        created_name.display_name()
    ))
}

fn list_collections(
    database_override: Option<std::path::PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteCollectionStore::open_existing(&database_path)?;
    let use_case = ListCollections::new(store);
    let collections = use_case.execute()?;

    Ok(collections
        .iter()
        .map(CollectionName::display_name)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn destroy_collection(
    raw_name: &str,
    database_override: Option<std::path::PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
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
    home_directory: &Path,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
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
    paths: &[PathBuf],
    database_override: Option<PathBuf>,
    force: bool,
    home_directory: &Path,
) -> Result<String, AppError> {
    let name = CollectionName::try_from(raw_name)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let mut use_case = UpdateCollection::new(SystemFileSystem, store, SystemClock);
    let outcome = use_case.execute(&name, UpdateTarget::Paths(paths), force)?;

    Ok(format_update(name.display_name(), &outcome))
}

fn update_all_collections(
    database_override: Option<PathBuf>,
    force: bool,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let collection_store = SqliteCollectionStore::open_existing(&database_path)?;
    let names = ListCollections::new(collection_store).execute()?;

    let store = SqliteFileStore::open_for_ingestion(&database_path)?;
    let mut use_case = UpdateCollection::new(SystemFileSystem, store, SystemClock);

    let mut lines = Vec::new();
    for name in &names {
        let outcome = use_case.execute(name, UpdateTarget::Stored, force)?;
        lines.push(format_update(name.display_name(), &outcome));
    }

    Ok(lines.join("\n"))
}

fn index_status(
    database_override: Option<PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteLexicalIndexStore::open(&database_path)?;
    let use_case = ReadIndexStatus::new(store);
    let statuses = use_case.execute()?;

    Ok(statuses
        .iter()
        .map(render_index_status)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn search(args: &SearchArgs, home_directory: &Path) -> Result<String, AppError> {
    if args.query.trim().is_empty() {
        return Err(AppError::Search(kv_application::SearchError::EmptyQuery));
    }

    let database_path = args
        .database
        .clone()
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
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

fn hybrid(args: &HybridArgs, home_directory: &Path) -> Result<String, AppError> {
    if args.query.trim().is_empty() {
        return Err(AppError::Hybrid(kv_application::HybridError::EmptyQuery));
    }

    let database_path = args
        .database
        .clone()
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let generator = FastembedGenerator::new(None);
    let reranker = FastembedReranker::new(None);
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
    home_directory: &Path,
) -> Result<String, AppError> {
    let collection = CollectionName::try_from(raw_collection)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteFileRetrievalStore::open(&database_path)?;
    let use_case = GetFile::new(store);
    let file = use_case.execute(&collection, name_or_id)?;

    String::from_utf8(file.content().to_vec()).map_err(|_| AppError::NonUtf8Content)
}

fn graph_neighbors(
    raw_node: &str,
    collection_name: Option<&str>,
    database_override: Option<PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
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
    home_directory: &Path,
) -> Result<String, AppError> {
    let _collection = CollectionName::try_from(collection_name)?;
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
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

fn render_human(set: &SearchResultSet, related: Option<&[Vec<RelatedFile>]>) -> String {
    let mut lines = Vec::new();
    for (index, result) in set.results().iter().enumerate() {
        let position = result.position();
        let header = if position.line_start() == 0 {
            format!(
                "{}. {} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                result.kind().as_str(),
                result.score()
            )
        } else {
            format!(
                "{}. {}:{}-{} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                position.line_start(),
                position.line_end(),
                result.kind().as_str(),
                result.score()
            )
        };
        lines.push(header);
        lines.push(result.text().to_owned());
        render_related_lines(&mut lines, related, index);
    }
    if !set.results().is_empty() {
        lines.push(format!("{} match(es)", set.total()));
    }
    lines.join("\n")
}

/// Appends the `related: <path> (<RELATION>)` lines for the result at `index`.
fn render_related_lines(
    lines: &mut Vec<String>,
    related: Option<&[Vec<RelatedFile>]>,
    index: usize,
) {
    let Some(related) = related else {
        return;
    };
    for file in related.get(index).into_iter().flatten() {
        lines.push(format!(
            "related: {} ({})",
            file.path().display(),
            file.relation().as_str()
        ));
    }
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

fn render_json(
    set: &SearchResultSet,
    query: &str,
    scope: &str,
    limit: u16,
    related: Option<&[Vec<RelatedFile>]>,
) -> String {
    let results: Vec<serde_json::Value> = set
        .results()
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let position = result.position();
            let mut value = serde_json::json!({
                "collection": result.collection().display_name(),
                "path": result.path().to_string_lossy(),
                "kind": result.kind().as_str(),
                "text": result.text(),
                "score": result.score(),
                "position": {
                    "byte_offset": position.byte_offset(),
                    "byte_length": position.byte_length(),
                    "line_start": position.line_start(),
                    "line_end": position.line_end(),
                },
            });
            append_related_field(&mut value, related, index);
            value
        })
        .collect();

    serde_json::json!({
        "query": query,
        "scope": scope,
        "limit": limit,
        "total": set.total(),
        "results": results,
    })
    .to_string()
}

/// Adds the `related` field to a result JSON object when context is present.
fn append_related_field(
    value: &mut serde_json::Value,
    related: Option<&[Vec<RelatedFile>]>,
    index: usize,
) {
    let Some(related) = related else {
        return;
    };
    let entries: Vec<serde_json::Value> = related
        .get(index)
        .into_iter()
        .flatten()
        .map(|file| {
            serde_json::json!({
                "path": file.path().to_string_lossy(),
                "relation": file.relation().as_str(),
            })
        })
        .collect();
    value["related"] = serde_json::json!(entries);
}

fn render_hybrid_human(set: &HybridResultSet, related: Option<&[Vec<RelatedFile>]>) -> String {
    let mut lines = Vec::new();
    for (index, result) in set.results().iter().enumerate() {
        let position = result.position();
        let header = if position.line_start() == 0 {
            format!(
                "{}. {} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                result.kind().as_str(),
                result.ordering_score()
            )
        } else {
            format!(
                "{}. {}:{}-{} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                position.line_start(),
                position.line_end(),
                result.kind().as_str(),
                result.ordering_score()
            )
        };
        lines.push(header);
        lines.push(result.text().to_owned());
        render_related_lines(&mut lines, related, index);
    }
    if !set.results().is_empty() {
        lines.push(format!("{} result(s)", set.results().len()));
    }
    if set.rerank_warning() {
        lines.push("re-ranking skipped: re-ranker model is not cached; pass --no-rerank to suppress this warning".to_owned());
    }
    lines.join("\n")
}

fn render_hybrid_json(
    set: &HybridResultSet,
    query: &str,
    scope: &str,
    limit: u16,
    related: Option<&[Vec<RelatedFile>]>,
) -> String {
    let results: Vec<serde_json::Value> = set
        .results()
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let position = result.position();
            let mut value = serde_json::json!({
                "collection": result.collection().display_name(),
                "path": result.path().to_string_lossy(),
                "kind": result.kind().as_str(),
                "text": result.text(),
                "reranker_score": result.rerank_score(),
                "fused_score": result.fused_score(),
                "bm25_score": result.lexical_score(),
                "cosine_similarity": result.semantic_score(),
                "ordering_score": result.ordering_score(),
                "position": {
                    "byte_offset": position.byte_offset(),
                    "byte_length": position.byte_length(),
                    "line_start": position.line_start(),
                    "line_end": position.line_end(),
                },
            });
            append_related_field(&mut value, related, index);
            value
        })
        .collect();

    serde_json::json!({
        "query": query,
        "scope": scope,
        "limit": limit,
        "reranked": set.reranked(),
        "rerank_warning": set.rerank_warning(),
        "total": results.len(),
        "results": results,
    })
    .to_string()
}

fn embed(
    collection_name: Option<&str>,
    model_name: Option<&str>,
    reranker_name: Option<&str>,
    download: bool,
    database_override: Option<PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let generator = FastembedGenerator::new(None);
    let reranker = FastembedReranker::new(None);
    let store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let mut use_case = EmbedCollections::new(generator, store, SystemClock, reranker);

    let model = model_name.map(EmbeddingModel::try_from).transpose()?;
    let reranker = reranker_name.map(RerankerModel::try_from).transpose()?;
    let collection = collection_name.map(CollectionName::try_from).transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => EmbedScope::Collection(collection),
        None => EmbedScope::All,
    };

    let report = use_case.execute(scope, model.as_ref(), reranker.as_ref(), download)?;

    let rendered = render_embed_report(&report);
    if report.any_failed() {
        Err(AppError::EmbedPartial(rendered))
    } else {
        Ok(rendered)
    }
}

fn render_embed_report(report: &EmbedReport) -> String {
    let mut lines = report
        .outcomes()
        .iter()
        .map(render_embed_outcome)
        .collect::<Vec<_>>();
    if report.any_failed() {
        lines.push("embedding completed with failures".to_owned());
    }
    lines.join("\n")
}

fn render_embed_outcome(outcome: &EmbedOutcome) -> String {
    let name = outcome.collection().display_name();
    match outcome {
        EmbedOutcome::Embedded { passage_count, .. } => {
            format!("collection \"{name}\": embedded {passage_count} passage(s)")
        }
        EmbedOutcome::AlreadyCurrent { .. } => {
            format!("collection \"{name}\": already current")
        }
        EmbedOutcome::Skipped {
            reason: SkipReason::NoFiles,
            ..
        } => format!("collection \"{name}\": skipped (no files)"),
        EmbedOutcome::Skipped {
            reason: SkipReason::LexicalNotBuilt,
            ..
        } => format!("collection \"{name}\": skipped (lexical index not built)"),
        EmbedOutcome::Failed { message, .. } => {
            format!("collection \"{name}\": failed ({message})")
        }
    }
}

fn render_index_status(status: &IndexStatus) -> String {
    match (status.state(), status.built_at()) {
        (IndexState::Built, Some(timestamp)) => format!(
            "collection \"{}\": lexical index built, {} file(s), {} passage(s), built at {}",
            status.collection().display_name(),
            status.file_count(),
            status.passage_count(),
            timestamp.as_unix_seconds()
        ),
        _ => format!(
            "collection \"{}\": lexical index not built",
            status.collection().display_name()
        ),
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use kv_application::{
        EmbedOutcome, EmbedReport, Position, SearchResult, SearchResultSet, SkipReason,
    };
    use kv_domain::{CollectionName, PassageKind, RelationKind};

    use crate::related::RelatedFile;

    use super::{
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
