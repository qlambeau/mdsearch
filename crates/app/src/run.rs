use std::ffi::OsString;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use kv_application::{
    AddFiles, CreateCollection, DestroyCollection, IndexState, IndexStatus, ListCollections,
    ReadIndexStatus, SearchLexical, SearchResultSet, SearchScope, UpdateCollection, UpdateOutcome,
    UpdateTarget,
};
use kv_domain::CollectionName;
use kv_infrastructure::{SystemClock, SystemFileSystem};
use kv_store_sqlite::{
    SqliteCollectionStore, SqliteFileStore, SqliteLexicalIndexStore, SqliteLexicalSearchStore,
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

    Ok(render_search_results(&set))
}

fn render_search_results(set: &SearchResultSet) -> String {
    let mut lines = Vec::new();
    for (index, result) in set.results().iter().enumerate() {
        lines.push(format!(
            "{}. {} ({}, score {:.3})",
            index + 1,
            result.path().display(),
            result.kind().as_str(),
            result.score()
        ));
        lines.push(result.text().to_owned());
    }
    if !set.results().is_empty() {
        lines.push(format!("{} match(es)", set.total()));
    }
    lines.join("\n")
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
