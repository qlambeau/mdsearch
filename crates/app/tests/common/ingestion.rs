use kv_application::AddFiles;
use kv_infrastructure::{SystemClock, SystemFileSystem};
use kv_store_sqlite::SqliteFileStore;
use std::fmt::Write;
use std::path::{Path, PathBuf};

/// Builds a stored-but-unindexed fixture through the ingestion application port.
/// The removed CLI ingestion command is never invoked.
pub(crate) fn ingest(
    home: &Path,
    collection: &str,
    paths: &[&str],
    skip: bool,
    database: Option<&str>,
) -> Result<String, kv_app::AppError> {
    let database = database.map_or_else(|| home.join(".mdsearch/collections.db"), PathBuf::from);
    let store = SqliteFileStore::open_for_ingestion(&database)?;
    let files = paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    let outcome = AddFiles::new(SystemFileSystem, store, SystemClock).execute(
        &kv_domain::CollectionName::try_from(collection)?,
        &files,
        skip,
    )?;
    let mut message = format!(
        "added {} {} to collection \"{collection}\"",
        outcome.added(),
        if outcome.added() == 1 {
            "file"
        } else {
            "files"
        }
    );
    if outcome.skipped() > 0 {
        write!(message, " (skipped {})", outcome.skipped())
            .map_err(|error| kv_app::AppError::Model(error.to_string()))?;
    }
    Ok(message)
}
