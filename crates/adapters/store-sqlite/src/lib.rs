#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `SQLite` adapter for the `kv` application ports.

use std::error::Error;
use std::fs;
use std::path::Path;

use kv_application::{CollectionStore, CollectionStoreError};
use kv_domain::{CollectionName, Timestamp};
use rusqlite::{Connection, OptionalExtension, params};
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};

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

        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER NOT NULL
                );
                INSERT INTO schema_version(version)
                SELECT 1
                WHERE NOT EXISTS (SELECT 1 FROM schema_version);
                CREATE TABLE IF NOT EXISTS collections (
                    collection_id INTEGER PRIMARY KEY,
                    display_name TEXT NOT NULL,
                    name_key TEXT NOT NULL UNIQUE,
                    created_at INTEGER NOT NULL
                );",
            )
            .map_err(storage_failure)?;

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
}

fn database_unavailable(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::DatabaseUnavailable(Box::new(error))
}

fn storage_failure(error: impl Error + Send + Sync + 'static) -> CollectionStoreError {
    CollectionStoreError::Storage(Box::new(error))
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
