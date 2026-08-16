#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `SQLite` adapter for the `mdsearch` application ports.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use kv_application::{
    CollectionStore, CollectionStoreError, FileRecord, FileStore, FileStoreError, StoredFile,
};
use kv_domain::{CollectionName, ContentHash, Timestamp};
use rusqlite::{Connection, OptionalExtension, params};
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};

/// The current database schema version applied by [`migrate`].
const CURRENT_SCHEMA_VERSION: i64 = 2;

/// Persists collection metadata in one `SQLite` database file.
pub struct SqliteCollectionStore {
    connection: Connection,
}

impl SqliteCollectionStore {
    /// Opens or initializes a `SQLite` collection database at `path`.
    ///
    /// # Errors
    ///
    /// Returns a database-unavailable error when the path cannot be created or
    /// opened, or a storage error when schema initialization fails.
    pub fn open(path: &Path) -> Result<Self, CollectionStoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(database_unavailable)?;
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        register_vector_extension(&connection).map_err(storage_failure)?;
        migrate(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }

    /// Opens an existing `SQLite` collection database at `path` without
    /// creating or initializing it.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, or a
    /// database-unavailable error when it cannot be opened.
    pub fn open_existing(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(database_unavailable)?;

        Ok(Self { connection })
    }
}

/// Persists ingested files in one `SQLite` database file.
pub struct SqliteFileStore {
    connection: Connection,
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

        migrate(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }
}

fn register_vector_extension(connection: &Connection) -> Result<(), sqlite3_ext::Error> {
    let extension_connection = ExtensionConnection::from_rusqlite(connection);
    let module = StandardModule::<VectorTable<'_>>::new()
        .with_update()
        .with_transactions()
        .with_find_function();
    let registry = Registry::default();

    extension_connection.create_module("vector", module, registry.clone())?;
    scalar::register_scalar_functions(extension_connection, registry)
}

/// Creates the schema tables if absent and bumps the stored version to
/// [`CURRENT_SCHEMA_VERSION`].
fn migrate(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS collections (
            collection_id INTEGER PRIMARY KEY,
            display_name TEXT NOT NULL,
            name_key TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS files (
            file_id INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            content BLOB NOT NULL,
            content_hash TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(collection_id, path)
        );",
    )?;

    let version: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if version < CURRENT_SCHEMA_VERSION {
        connection.execute("DELETE FROM schema_version", [])?;
        connection.execute(
            "INSERT INTO schema_version(version) VALUES (?1)",
            [CURRENT_SCHEMA_VERSION],
        )?;
    }

    Ok(())
}

impl CollectionStore for SqliteCollectionStore {
    fn create_collection(
        &mut self,
        name: &CollectionName,
        created_at: Timestamp,
    ) -> Result<(), CollectionStoreError> {
        let created_at = i64::try_from(created_at.as_unix_seconds()).map_err(storage_failure)?;
        let transaction = self.connection.transaction().map_err(storage_failure)?;

        let duplicate = transaction
            .query_row(
                "SELECT 1 FROM collections WHERE name_key = ?1 LIMIT 1",
                params![name.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_failure)?
            .is_some();

        if duplicate {
            return Err(CollectionStoreError::Duplicate);
        }

        transaction
            .execute(
                "INSERT INTO collections(display_name, name_key, created_at)
                 VALUES (?1, ?2, ?3)",
                params![name.display_name(), name.name_key(), created_at],
            )
            .map_err(storage_failure)?;

        transaction.commit().map_err(storage_failure)
    }

    fn list_collections(&self) -> Result<Vec<CollectionName>, CollectionStoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT display_name FROM collections ORDER BY name_key")
            .map_err(storage_failure)?;

        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(storage_failure)?;

        let mut names = Vec::new();
        for row in rows {
            let display_name = row.map_err(storage_failure)?;
            let name = CollectionName::try_from(display_name.as_str()).map_err(storage_failure)?;
            names.push(name);
        }

        Ok(names)
    }

    fn destroy_collection(
        &mut self,
        name: &CollectionName,
    ) -> Result<CollectionName, CollectionStoreError> {
        let display_name = self
            .connection
            .query_row(
                "DELETE FROM collections WHERE name_key = ?1 RETURNING display_name",
                params![name.name_key()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_failure)?;

        let display_name = display_name.ok_or(CollectionStoreError::CollectionNotFound)?;

        CollectionName::try_from(display_name.as_str()).map_err(storage_failure)
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
    ) -> Result<(), FileStoreError> {
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

        transaction.commit().map_err(file_storage_failure)
    }
}

fn resolve_collection_id(
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

fn upsert_file(
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

fn database_unavailable(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::DatabaseUnavailable(Box::new(error))
}

fn storage_failure(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::Storage(Box::new(error))
}

fn file_storage_failure(error: impl Error + Send + Sync + 'static) -> FileStoreError {
    FileStoreError::Storage(Box::new(error))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rusqlite::Connection;

    use super::register_vector_extension;

    #[test]
    fn registers_vector_tables_and_nearest_neighbor_queries() -> Result<(), Box<dyn Error>> {
        let connection = Connection::open_in_memory()?;

        register_vector_extension(&connection)?;
        connection.execute_batch(
            "CREATE VIRTUAL TABLE embeddings USING vector(
                dim=3,
                type=float4,
                metric=cosine
            );
            INSERT INTO embeddings(vector)
            VALUES (vector_from_json('[1.0, 0.0, 0.0]', 'float4'));
            INSERT INTO embeddings(vector)
            VALUES (vector_from_json('[0.0, 1.0, 0.0]', 'float4'));",
        )?;

        let nearest_id: i64 = connection.query_row(
            "SELECT rowid
             FROM embeddings
             WHERE knn_match(distance, vector_from_json('[0.9, 0.1, 0.0]', 'float4'))
             LIMIT 1",
            [],
            |row| row.get(0),
        )?;

        assert_eq!(nearest_id, 1);

        Ok(())
    }
}
