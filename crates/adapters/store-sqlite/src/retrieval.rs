//! `SQLite` retrieval persistence.

use std::path::{Path, PathBuf};

use kv_application::{FileRetrievalStore, FileRetrievalStoreError, RetrievedFile};
use kv_domain::{CollectionName, FileId};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{SqliteFileStore, retrieval_storage_failure};

/// Retrieval view of the same connection used by [`SqliteFileStore`].
pub type SqliteFileRetrievalStore = SqliteFileStore;

impl FileRetrievalStore for SqliteFileRetrievalStore {
    fn get_by_path(
        &self,
        collection: &CollectionName,
        path: &Path,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError> {
        let collection_id = resolve_retrieval_collection_id(&self.connection, collection)?;
        let path = path.to_string_lossy();
        self.connection
            .query_row(
                "SELECT path, content FROM files
                 WHERE collection_id = ?1 AND path = ?2 LIMIT 1",
                params![collection_id, path.as_ref()],
                retrieval_file_row,
            )
            .optional()
            .map_err(retrieval_storage_failure)
    }

    fn get_by_id(
        &self,
        collection: &CollectionName,
        id: FileId,
    ) -> Result<Option<RetrievedFile>, FileRetrievalStoreError> {
        let collection_id = resolve_retrieval_collection_id(&self.connection, collection)?;
        let id = i64::try_from(id.as_u64()).map_err(retrieval_storage_failure)?;
        self.connection
            .query_row(
                "SELECT path, content FROM files
                 WHERE collection_id = ?1 AND file_id = ?2 LIMIT 1",
                params![collection_id, id],
                retrieval_file_row,
            )
            .optional()
            .map_err(retrieval_storage_failure)
    }

    fn list_by_basename(
        &self,
        collection: &CollectionName,
        basename: &str,
    ) -> Result<Vec<RetrievedFile>, FileRetrievalStoreError> {
        let collection_id = resolve_retrieval_collection_id(&self.connection, collection)?;
        let mut statement = self
            .connection
            .prepare("SELECT path, content FROM files WHERE collection_id = ?1")
            .map_err(retrieval_storage_failure)?;
        let rows = statement
            .query_map(params![collection_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(retrieval_storage_failure)?;

        let mut matches = Vec::new();
        for row in rows {
            let (path, content) = row.map_err(retrieval_storage_failure)?;
            let path = PathBuf::from(path);
            let is_match = path.file_name().and_then(|name| name.to_str()) == Some(basename);
            if is_match {
                matches.push(RetrievedFile::new(path, content));
            }
        }

        Ok(matches)
    }
}

pub(super) fn resolve_retrieval_collection_id(
    connection: &Connection,
    collection: &CollectionName,
) -> Result<i64, FileRetrievalStoreError> {
    connection
        .query_row(
            "SELECT collection_id FROM collections WHERE name_key = ?1 LIMIT 1",
            params![collection.name_key()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(retrieval_storage_failure)?
        .ok_or(FileRetrievalStoreError::CollectionNotFound)
}

pub(super) fn retrieval_file_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RetrievedFile> {
    Ok(RetrievedFile::new(
        PathBuf::from(row.get::<_, String>(0)?),
        row.get::<_, Vec<u8>>(1)?,
    ))
}
