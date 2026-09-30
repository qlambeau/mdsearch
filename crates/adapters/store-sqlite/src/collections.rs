//! `SQLite` collections persistence.

use std::fs;
use std::path::{Path, PathBuf};

use kv_application::{
    CollectionSourceStore, CollectionSourceSummary, CollectionStore, CollectionStoreError,
};
use kv_domain::{CollectionName, CollectionSource, SourceKind, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::{
    database_unavailable, migrate, register_vector_extension, schema_version, storage_failure,
};

/// Persists collection metadata in one `SQLite` database file.
pub struct SqliteCollectionStore {
    pub(super) connection: Connection,
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

        register_vector_extension(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }
}

impl SqliteCollectionStore {
    /// Returns whether the `embeddings` vector table exists in the database.
    fn embeddings_table_exists(&self) -> Result<bool, rusqlite::Error> {
        self.connection.query_row(
            "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = 'embeddings'
                )",
            [],
            |row| row.get(0),
        )
    }
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
        let collection_id = self
            .connection
            .query_row(
                "SELECT collection_id FROM collections WHERE name_key = ?1 LIMIT 1",
                params![name.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_failure)?
            .ok_or(CollectionStoreError::CollectionNotFound)?;

        let embeddings_table_exists = self.embeddings_table_exists().map_err(storage_failure)?;
        let transaction = self.connection.transaction().map_err(storage_failure)?;

        if embeddings_table_exists {
            transaction
                .execute(
                    "DELETE FROM embeddings WHERE collection_id = ?1",
                    params![collection_id],
                )
                .map_err(storage_failure)?;
        }
        transaction
            .execute(
                "DELETE FROM passages WHERE rowid IN (
                     SELECT passage_rowid FROM passage_files WHERE collection_id = ?1
                 )",
                params![collection_id],
            )
            .map_err(storage_failure)?;
        for table in [
            "passage_files",
            "collection_sources",
            "files",
            "edges",
            "nodes",
            "graph_state",
            "lexical_index_state",
            "semantic_index_state",
        ] {
            transaction
                .execute(
                    &format!("DELETE FROM {table} WHERE collection_id = ?1"),
                    params![collection_id],
                )
                .map_err(storage_failure)?;
        }
        let display_name = transaction
            .query_row(
                "DELETE FROM collections WHERE name_key = ?1 RETURNING display_name",
                params![name.name_key()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_failure)?;
        let display_name = display_name.ok_or(CollectionStoreError::CollectionNotFound)?;

        transaction.commit().map_err(storage_failure)?;

        CollectionName::try_from(display_name.as_str()).map_err(storage_failure)
    }
}

impl CollectionSourceStore for SqliteCollectionStore {
    fn create_collection_with_sources(
        &mut self,
        name: &CollectionName,
        created_at: Timestamp,
        sources: &[CollectionSource],
    ) -> Result<(), CollectionStoreError> {
        let created_at = i64::try_from(created_at.as_unix_seconds()).map_err(storage_failure)?;
        let transaction = self.connection.transaction().map_err(storage_failure)?;
        transaction
            .execute(
                "INSERT INTO collections(display_name, name_key, created_at) VALUES (?1, ?2, ?3)",
                params![name.display_name(), name.name_key(), created_at],
            )
            .map_err(|error| {
                if is_unique_violation(&error) {
                    CollectionStoreError::Duplicate
                } else {
                    storage_failure(error)
                }
            })?;
        let collection_id = transaction.last_insert_rowid();
        for source in sources {
            insert_source(&transaction, collection_id, source).map_err(storage_failure)?;
        }
        transaction.commit().map_err(storage_failure)
    }

    fn replace_collection_sources(
        &mut self,
        name: &CollectionName,
        sources: &[CollectionSource],
    ) -> Result<(), CollectionStoreError> {
        let transaction = self.connection.transaction().map_err(storage_failure)?;
        let collection_id = transaction
            .query_row(
                "SELECT collection_id FROM collections WHERE name_key = ?1",
                params![name.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_failure)?
            .ok_or(CollectionStoreError::CollectionNotFound)?;
        transaction
            .execute(
                "DELETE FROM collection_sources WHERE collection_id = ?1",
                params![collection_id],
            )
            .map_err(storage_failure)?;
        for source in sources {
            insert_source(&transaction, collection_id, source).map_err(storage_failure)?;
        }
        transaction.commit().map_err(storage_failure)
    }

    fn collection_sources(
        &self,
        name: &CollectionName,
    ) -> Result<Vec<CollectionSource>, CollectionStoreError> {
        let collection_id = self
            .connection
            .query_row(
                "SELECT collection_id FROM collections WHERE name_key = ?1",
                params![name.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(storage_failure)?
            .ok_or(CollectionStoreError::CollectionNotFound)?;
        if schema_version(&self.connection).map_err(storage_failure)? < 8 {
            return Ok(Vec::new());
        }
        load_sources(&self.connection, collection_id).map_err(storage_failure)
    }

    fn list_collections_with_sources(
        &self,
    ) -> Result<Vec<CollectionSourceSummary>, CollectionStoreError> {
        let version = schema_version(&self.connection).map_err(storage_failure)?;
        let policy = if version >= 9 {
            "semantic_enabled"
        } else if version >= 5 {
            "EXISTS(SELECT 1 FROM semantic_index_state s WHERE s.collection_id = collections.collection_id)"
        } else {
            "0"
        };
        let sql = format!(
            "SELECT collection_id, display_name, {policy} FROM collections ORDER BY name_key"
        );
        let mut statement = self.connection.prepare(&sql).map_err(storage_failure)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(storage_failure)?;
        let mut summaries = Vec::new();
        for row in rows {
            let (id, display_name, semantic_enabled) = row.map_err(storage_failure)?;
            summaries.push(CollectionSourceSummary {
                name: CollectionName::try_from(display_name.as_str()).map_err(storage_failure)?,
                sources: if version >= 8 {
                    load_sources(&self.connection, id).map_err(storage_failure)?
                } else {
                    Vec::new()
                },
                semantic_enabled,
            });
        }
        Ok(summaries)
    }
}

pub(super) fn insert_source(
    transaction: &Transaction<'_>,
    collection_id: i64,
    source: &CollectionSource,
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "INSERT INTO collection_sources(collection_id, source_path, source_kind) VALUES (?1, ?2, ?3)",
        params![collection_id, source.path().to_string_lossy(), match source.kind() { SourceKind::File => "file", SourceKind::Directory => "directory" }],
    )?;
    Ok(())
}

pub(super) fn load_sources(
    connection: &Connection,
    collection_id: i64,
) -> Result<Vec<CollectionSource>, rusqlite::Error> {
    let mut statement = connection.prepare("SELECT source_path, source_kind FROM collection_sources WHERE collection_id = ?1 ORDER BY source_path")?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut sources = Vec::new();
    for row in rows {
        let (path, kind) = row?;
        let kind = match kind.as_str() {
            "file" => SourceKind::File,
            "directory" => SourceKind::Directory,
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        sources.push(CollectionSource::new(PathBuf::from(path), kind));
    }
    Ok(sources)
}

pub(super) fn is_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(error, rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ffi::ErrorCode::ConstraintViolation)
}
