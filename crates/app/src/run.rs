use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kv_application::{
    AddFiles, CreateCollection, DestroyCollection, EmbedCollections, EmbedOutcome, EmbedReport,
    EmbedScope, GetFile, IndexState, IndexStatus, ListCollections, ReadIndexStatus, SearchLexical,
    SearchResultSet, SearchScope, SkipReason, UpdateCollection, UpdateOutcome, UpdateTarget,
};
use kv_domain::{CollectionName, EmbeddingModel};
use kv_embed_fastembed::FastembedGenerator;
use kv_infrastructure::{SystemClock, SystemFileSystem};
use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileRetrievalStore, SqliteFileStore, SqliteLexicalIndexStore,
    SqliteLexicalSearchStore, SqliteSemanticIndexStore,
};

use crate::AppError;
use crate::cli::{Cli, CollectionCommand, Command, IndexCommand};

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
        Command::Search(arguments) => search(
            &arguments.query,
            arguments.collection.as_deref(),
            arguments.limit,
            arguments.json,
            arguments.database,
            home_directory,
        ),
        Command::Get(arguments) => get_file(
            &arguments.collection,
            &arguments.name_or_id,
            arguments.database,
            home_directory,
        ),
        Command::Embed(arguments) => embed(
            arguments.collection.as_deref(),
            arguments.model.as_deref(),
            arguments.download,
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

fn search(
    query: &str,
    collection_name: Option<&str>,
    limit: u16,
    json: bool,
    database_override: Option<PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    if query.trim().is_empty() {
        return Err(AppError::Search(kv_application::SearchError::EmptyQuery));
    }

    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let store = SqliteLexicalSearchStore::open(&database_path)?;
    let use_case = SearchLexical::new(store);

    let collection = collection_name.map(CollectionName::try_from).transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => SearchScope::Collection(collection),
        None => SearchScope::All,
    };

    let set = use_case.execute(query, usize::from(limit), scope)?;

    let scope_name = collection.as_ref().map_or_else(
        || "all".to_owned(),
        |collection| collection.display_name().to_owned(),
    );

    if json {
        Ok(render_json(&set, query, &scope_name, limit))
    } else {
        Ok(render_human(&set))
    }
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

fn render_human(set: &SearchResultSet) -> String {
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
    }
    if !set.results().is_empty() {
        lines.push(format!("{} match(es)", set.total()));
    }
    lines.join("\n")
}

fn render_json(set: &SearchResultSet, query: &str, scope: &str, limit: u16) -> String {
    let results: Vec<serde_json::Value> = set
        .results()
        .iter()
        .map(|result| {
            let position = result.position();
            serde_json::json!({
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
            })
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

fn embed(
    collection_name: Option<&str>,
    model_name: Option<&str>,
    download: bool,
    database_override: Option<PathBuf>,
    home_directory: &Path,
) -> Result<String, AppError> {
    let database_path = database_override
        .unwrap_or_else(|| home_directory.join(".mdsearch").join("collections.db"));
    let generator = FastembedGenerator::new(None);
    let store = SqliteSemanticIndexStore::open_for_embedding(&database_path)?;
    let mut use_case = EmbedCollections::new(generator, store, SystemClock);

    let model = model_name.map(EmbeddingModel::try_from).transpose()?;
    let collection = collection_name.map(CollectionName::try_from).transpose()?;
    let scope = match collection.as_ref() {
        Some(collection) => EmbedScope::Collection(collection),
        None => EmbedScope::All,
    };

    let report = use_case.execute(scope, model.as_ref(), download)?;

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
    use kv_application::{EmbedOutcome, EmbedReport, SkipReason};
    use kv_domain::CollectionName;

    use super::{render_embed_outcome, render_embed_report};

    fn collection(name: &str) -> Result<CollectionName, kv_domain::CollectionNameError> {
        CollectionName::try_from(name)
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
