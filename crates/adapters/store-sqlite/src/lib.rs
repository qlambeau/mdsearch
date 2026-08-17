#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! `SQLite` adapter for the `mdsearch` application ports.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use kv_application::{
    CollectionStore, CollectionStoreError, FileRecord, FileStore, FileStoreError, IndexStatus,
    IndexStoreError, LexicalIndexStore, LexicalSearchStore, ReconcileOutcome, SearchResult,
    SearchResultSet, SearchScope, SearchStoreError, StoredFile,
};
use kv_domain::{
    CollectionName, ContentHash, FrontmatterIssue, PassageKind, Timestamp, segment_passages,
};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};

/// The current database schema version applied by [`migrate`].
const CURRENT_SCHEMA_VERSION: i64 = 3;

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
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS passages USING fts5(
            content,
            tokenize = 'unicode61'
        );
        CREATE TABLE IF NOT EXISTS passage_files (
            passage_rowid INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
            file_id INTEGER NOT NULL,
            kind TEXT NOT NULL,
            position INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_passage_files_collection
            ON passage_files(collection_id);
        CREATE TABLE IF NOT EXISTS lexical_index_state (
            collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
            passage_count INTEGER NOT NULL,
            built_at INTEGER NOT NULL
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

        transaction.commit().map_err(file_storage_failure)?;

        Ok(ReconcileOutcome::new(malformed))
    }
}

/// Rebuilds the lexical index for a collection from its stored files.
///
/// Deletes the collection's existing passages and reinserts them from the
/// current `files` rows, then records the index state. Returns the number of
/// files whose frontmatter could not be parsed.
fn rebuild_index(
    transaction: &Transaction<'_>,
    collection_id: i64,
    built_at: i64,
) -> Result<usize, FileStoreError> {
    let old_rowids = {
        let mut statement = transaction
            .prepare("SELECT passage_rowid FROM passage_files WHERE collection_id = ?1")
            .map_err(file_storage_failure)?;
        let rows = statement
            .query_map(params![collection_id], |row| row.get::<_, i64>(0))
            .map_err(file_storage_failure)?;

        let mut rowids = Vec::new();
        for row in rows {
            rowids.push(row.map_err(file_storage_failure)?);
        }
        rowids
    };

    for rowid in old_rowids {
        transaction
            .execute("DELETE FROM passages WHERE rowid = ?1", params![rowid])
            .map_err(file_storage_failure)?;
    }

    transaction
        .execute(
            "DELETE FROM passage_files WHERE collection_id = ?1",
            params![collection_id],
        )
        .map_err(file_storage_failure)?;

    let mut statement = transaction
        .prepare("SELECT file_id, content FROM files WHERE collection_id = ?1 ORDER BY file_id")
        .map_err(file_storage_failure)?;
    let rows = statement
        .query_map(params![collection_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(file_storage_failure)?;

    let mut malformed = 0usize;
    let mut passage_count = 0i64;
    for row in rows {
        let (file_id, content) = row.map_err(file_storage_failure)?;
        let (passages, issue) = segment_passages(&content);
        if matches!(issue, Some(FrontmatterIssue::Malformed)) {
            malformed += 1;
        }
        for (position, passage) in passages.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO passages(content) VALUES (?1)",
                    params![passage.text()],
                )
                .map_err(file_storage_failure)?;
            let rowid = transaction.last_insert_rowid();
            let position = i64::try_from(position).map_err(file_storage_failure)?;
            transaction
                .execute(
                    "INSERT INTO passage_files(passage_rowid, collection_id, file_id, kind, position)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![rowid, collection_id, file_id, passage.kind().as_str(), position],
                )
                .map_err(file_storage_failure)?;
            passage_count += 1;
        }
    }

    transaction
        .execute(
            "INSERT INTO lexical_index_state(collection_id, passage_count, built_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(collection_id) DO UPDATE SET
                 passage_count = excluded.passage_count,
                 built_at = excluded.built_at",
            params![collection_id, passage_count, built_at],
        )
        .map_err(file_storage_failure)?;

    Ok(malformed)
}

/// Reads lexical index status from an existing `SQLite` database.
pub struct SqliteLexicalIndexStore {
    connection: Connection,
}

impl SqliteLexicalIndexStore {
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

impl LexicalIndexStore for SqliteLexicalIndexStore {
    fn status(&self) -> Result<Vec<IndexStatus>, IndexStoreError> {
        let has_index_tables = self.has_index_tables()?;
        let sql = if has_index_tables {
            "SELECT c.display_name,
                    COUNT(f.file_id) AS file_count,
                    COALESCE(s.passage_count, 0) AS passage_count,
                    s.built_at AS built_at
             FROM collections c
             LEFT JOIN files f ON f.collection_id = c.collection_id
             LEFT JOIN lexical_index_state s ON s.collection_id = c.collection_id
             GROUP BY c.collection_id
             ORDER BY c.name_key"
        } else {
            "SELECT c.display_name, COUNT(f.file_id) AS file_count,
                    0 AS passage_count, NULL AS built_at
             FROM collections c
             LEFT JOIN files f ON f.collection_id = c.collection_id
             GROUP BY c.collection_id
             ORDER BY c.name_key"
        };

        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(index_storage_failure)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })
            .map_err(index_storage_failure)?;

        let mut statuses = Vec::new();
        for row in rows {
            let (display_name, file_count, passage_count, built_at) =
                row.map_err(index_storage_failure)?;
            let collection =
                CollectionName::try_from(display_name.as_str()).map_err(index_storage_failure)?;
            statuses.push(IndexStatus::new(
                collection,
                usize::try_from(file_count).map_err(index_storage_failure)?,
                usize::try_from(passage_count).map_err(index_storage_failure)?,
                built_at
                    .and_then(|value| u64::try_from(value).ok())
                    .map(Timestamp::from_unix_seconds),
            ));
        }

        Ok(statuses)
    }
}

impl SqliteLexicalIndexStore {
    fn has_index_tables(&self) -> Result<bool, IndexStoreError> {
        schema_version(&self.connection)
            .map_err(index_storage_failure)
            .map(|version| version >= CURRENT_SCHEMA_VERSION)
    }
}

/// Searches the lexical index of an existing `SQLite` database.
pub struct SqliteLexicalSearchStore {
    connection: Connection,
}

impl SqliteLexicalSearchStore {
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

impl LexicalSearchStore for SqliteLexicalSearchStore {
    fn search(
        &self,
        query: &str,
        limit: usize,
        scope: SearchScope<'_>,
    ) -> Result<SearchResultSet, SearchStoreError> {
        let limit = i64::try_from(limit).map_err(search_storage_failure)?;

        let collection_id = match scope {
            SearchScope::All => None,
            SearchScope::Collection(collection) => {
                let collection_id = self
                    .resolve_collection_id(collection)
                    .map_err(search_storage_failure)?
                    .ok_or(SearchStoreError::CollectionNotFound)?;
                Some(collection_id)
            }
        };

        let built = schema_version(&self.connection).map_err(search_storage_failure)?
            >= CURRENT_SCHEMA_VERSION;
        if !built {
            return match collection_id {
                None => Ok(SearchResultSet::new(Vec::new(), 0)),
                Some(_) => Err(SearchStoreError::IndexNotBuilt),
            };
        }

        if let Some(collection_id) = collection_id
            && !self
                .index_is_built(collection_id)
                .map_err(search_storage_failure)?
        {
            return Err(SearchStoreError::IndexNotBuilt);
        }

        let results = self.search_results(query, collection_id, limit)?;
        let total = self.count_matches(query, collection_id)?;

        Ok(SearchResultSet::new(results, total))
    }
}

impl SqliteLexicalSearchStore {
    fn search_results(
        &self,
        query: &str,
        collection_id: Option<i64>,
        limit: i64,
    ) -> Result<Vec<SearchResult>, SearchStoreError> {
        let sql = "SELECT c.display_name, f.path, pf.kind, passages.content,
                          bm25(passages) AS rank
                   FROM passages
                   JOIN passage_files pf ON pf.passage_rowid = passages.rowid
                   JOIN files f ON f.file_id = pf.file_id
                   JOIN collections c ON c.collection_id = pf.collection_id
                   JOIN lexical_index_state s ON s.collection_id = c.collection_id
                   WHERE passages MATCH ?1 AND (?2 IS NULL OR c.collection_id = ?2)
                   ORDER BY rank ASC, c.name_key, f.path, pf.position
                   LIMIT ?3";

        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(search_storage_failure)?;
        let rows = statement
            .query_map(params![query, collection_id, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                ))
            })
            .map_err(search_query_failure)?;

        let mut results = Vec::new();
        for row in rows {
            let (display_name, path, kind, text, rank) = row.map_err(search_query_failure)?;
            let collection =
                CollectionName::try_from(display_name.as_str()).map_err(search_storage_failure)?;
            let kind = PassageKind::from_key(&kind).ok_or_else(|| {
                SearchStoreError::Storage(Box::new(std::io::Error::other("unknown passage kind")))
            })?;
            results.push(SearchResult::new(
                collection,
                PathBuf::from(path),
                kind,
                text,
                -rank,
            ));
        }

        Ok(results)
    }

    fn count_matches(
        &self,
        query: &str,
        collection_id: Option<i64>,
    ) -> Result<usize, SearchStoreError> {
        let sql = "SELECT COUNT(*)
                   FROM passages
                   JOIN passage_files pf ON pf.passage_rowid = passages.rowid
                   JOIN collections c ON c.collection_id = pf.collection_id
                   JOIN lexical_index_state s ON s.collection_id = c.collection_id
                   WHERE passages MATCH ?1 AND (?2 IS NULL OR c.collection_id = ?2)";

        let count: i64 = self
            .connection
            .query_row(sql, params![query, collection_id], |row| row.get(0))
            .map_err(search_query_failure)?;

        usize::try_from(count).map_err(search_storage_failure)
    }

    fn resolve_collection_id(
        &self,
        collection: &CollectionName,
    ) -> Result<Option<i64>, rusqlite::Error> {
        self.connection
            .query_row(
                "SELECT collection_id FROM collections WHERE name_key = ?1 LIMIT 1",
                params![collection.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
    }

    fn index_is_built(&self, collection_id: i64) -> Result<bool, rusqlite::Error> {
        self.connection
            .query_row(
                "SELECT 1 FROM lexical_index_state WHERE collection_id = ?1 LIMIT 1",
                params![collection_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.is_some())
    }
}

fn schema_version(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
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

fn index_storage_failure(error: impl Error + Send + Sync + 'static) -> IndexStoreError {
    IndexStoreError::Storage(Box::new(error))
}

fn search_storage_failure(error: impl Error + Send + Sync + 'static) -> SearchStoreError {
    SearchStoreError::Storage(Box::new(error))
}

/// Maps a search execution failure, distinguishing an FTS5 query problem.
fn search_query_failure(error: rusqlite::Error) -> SearchStoreError {
    match &error {
        rusqlite::Error::SqliteFailure(_, Some(message)) if message.contains("fts5") => {
            SearchStoreError::InvalidQuery {
                message: message.clone(),
            }
        }
        _ => SearchStoreError::Storage(Box::new(error)),
    }
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
