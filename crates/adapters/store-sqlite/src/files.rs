//! `SQLite` files persistence.

use std::path::{Path, PathBuf};

use kv_application::{
    CollectionStoreError, FileRecord, FileStore, FileStoreError, ReconcileOutcome,
    SemanticIndexStoreError, StoredFile,
};
use kv_domain::{CollectionName, ContentHash, Timestamp, file_set_fingerprint};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    database_unavailable, file_storage_failure, migrate, rebuild_graph, rebuild_index,
    register_vector_extension, semantic_storage_failure, storage_failure,
};

/// Persists ingested files in one `SQLite` database file.
pub struct SqliteFileStore {
    pub(super) connection: Connection,
}

impl SqliteFileStore {
    /// Opens an existing `SQLite` database for ingestion, migrating it to the
    /// current schema version.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, a
    /// database-unavailable error when it cannot be opened, or a storage error
    /// when the migration fails.
    pub fn open_for_ingestion(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        register_vector_extension(&connection).map_err(storage_failure)?;
        migrate(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }
}

impl FileStore for SqliteFileStore {
    fn upsert_files(
        &mut self,
        collection: &CollectionName,
        files: &[FileRecord],
        ingested_at: Timestamp,
    ) -> Result<(), FileStoreError> {
        let ingested_at =
            i64::try_from(ingested_at.as_unix_seconds()).map_err(file_storage_failure)?;
        let transaction = self
            .connection
            .transaction()
            .map_err(file_storage_failure)?;

        let collection_id = resolve_collection_id(&transaction, collection)?;

        for file in files {
            upsert_file(&transaction, collection_id, file, ingested_at)?;
        }

        transaction.commit().map_err(file_storage_failure)
    }

    fn list_files(&self, collection: &CollectionName) -> Result<Vec<StoredFile>, FileStoreError> {
        let collection_id = resolve_collection_id(&self.connection, collection)?;

        let mut statement = self
            .connection
            .prepare("SELECT path, content_hash FROM files WHERE collection_id = ?1 ORDER BY path")
            .map_err(file_storage_failure)?;

        let rows = statement
            .query_map(params![collection_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(file_storage_failure)?;

        let mut stored = Vec::new();
        for row in rows {
            let (path, hash) = row.map_err(file_storage_failure)?;
            let content_hash = ContentHash::try_from_hex(&hash).map_err(file_storage_failure)?;
            stored.push(StoredFile::new(PathBuf::from(path), content_hash));
        }

        Ok(stored)
    }

    fn reconcile(
        &mut self,
        collection: &CollectionName,
        upsert: &[FileRecord],
        delete: &[PathBuf],
        ingested_at: Timestamp,
    ) -> Result<ReconcileOutcome, FileStoreError> {
        let ingested_at =
            i64::try_from(ingested_at.as_unix_seconds()).map_err(file_storage_failure)?;
        let transaction = self
            .connection
            .transaction()
            .map_err(file_storage_failure)?;

        let collection_id = resolve_collection_id(&transaction, collection)?;

        for file in upsert {
            upsert_file(&transaction, collection_id, file, ingested_at)?;
        }

        for path in delete {
            let path = path.to_string_lossy();
            transaction
                .execute(
                    "DELETE FROM files WHERE collection_id = ?1 AND path = ?2",
                    params![collection_id, path.as_ref()],
                )
                .map_err(file_storage_failure)?;
        }

        let malformed = rebuild_index(&transaction, collection_id, ingested_at)?;
        rebuild_graph(&transaction, collection_id, ingested_at)?;
        let semantic_enabled = transaction
            .query_row(
                "SELECT semantic_enabled FROM collections WHERE collection_id = ?1",
                params![collection_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(file_storage_failure)?;
        if semantic_enabled == 0 {
            let embeddings_exist: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'embeddings')",
                    [],
                    |row| row.get(0),
                )
                .map_err(file_storage_failure)?;
            if embeddings_exist {
                transaction
                    .execute(
                        "DELETE FROM embeddings WHERE collection_id = ?1",
                        params![collection_id],
                    )
                    .map_err(file_storage_failure)?;
            }
            transaction
                .execute(
                    "DELETE FROM semantic_index_state WHERE collection_id = ?1",
                    params![collection_id],
                )
                .map_err(file_storage_failure)?;
        }

        transaction.commit().map_err(file_storage_failure)?;

        Ok(ReconcileOutcome::new(malformed))
    }
}

impl SqliteFileStore {
    /// Opens an existing database for embedding, migrating it to the current
    /// schema version.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, a
    /// database-unavailable error when it cannot be opened, or a storage error
    /// when the migration fails.
    pub fn open_for_embedding(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        register_vector_extension(&connection).map_err(storage_failure)?;
        migrate(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }
}

impl SqliteFileStore {
    /// Opens an existing database without creating or initializing it.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, or a
    /// database-unavailable error when it cannot be opened.
    pub fn open(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        Ok(Self { connection })
    }
}

pub(super) fn resolve_collection_id(
    connection: &Connection,
    collection: &CollectionName,
) -> Result<i64, FileStoreError> {
    connection
        .query_row(
            "SELECT collection_id FROM collections WHERE name_key = ?1 LIMIT 1",
            params![collection.name_key()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(file_storage_failure)?
        .ok_or(FileStoreError::CollectionNotFound)
}

pub(super) fn fingerprint_for_collection(
    connection: &Connection,
    collection_id: i64,
) -> Result<ContentHash, SemanticIndexStoreError> {
    let mut statement = connection
        .prepare("SELECT path, content_hash FROM files WHERE collection_id = ?1 ORDER BY path")
        .map_err(semantic_storage_failure)?;
    let rows = statement
        .query_map(params![collection_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(semantic_storage_failure)?;
    let mut files = Vec::new();
    for row in rows {
        let (path, hash) = row.map_err(semantic_storage_failure)?;
        files.push((
            PathBuf::from(path),
            ContentHash::try_from_hex(&hash).map_err(semantic_storage_failure)?,
        ));
    }
    let paths = files
        .iter()
        .map(|(path, hash)| (path.as_path(), hash))
        .collect::<Vec<_>>();
    Ok(file_set_fingerprint(&paths))
}

pub(super) fn upsert_file(
    connection: &Connection,
    collection_id: i64,
    file: &FileRecord,
    ingested_at: i64,
) -> Result<(), FileStoreError> {
    let path = file.path().to_string_lossy();
    let byte_size = i64::try_from(file.content().len()).map_err(file_storage_failure)?;

    connection
        .execute(
            "INSERT INTO files(
                collection_id, path, content, content_hash, byte_size, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(collection_id, path) DO UPDATE SET
                content = excluded.content,
                content_hash = excluded.content_hash,
                byte_size = excluded.byte_size,
                updated_at = excluded.updated_at",
            params![
                collection_id,
                path.as_ref(),
                file.content(),
                file.content_hash().as_str(),
                byte_size,
                ingested_at,
            ],
        )
        .map_err(file_storage_failure)?;

    Ok(())
}
