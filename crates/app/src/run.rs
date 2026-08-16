use std::ffi::OsString;
use std::path::Path;

use clap::Parser;
use kv_application::{CreateCollection, DestroyCollection, ListCollections};
use kv_domain::CollectionName;
use kv_infrastructure::SystemClock;
use kv_store_sqlite::SqliteCollectionStore;

use crate::AppError;
use crate::cli::{Cli, CollectionCommand, Command};

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
