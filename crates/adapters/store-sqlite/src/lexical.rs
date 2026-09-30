//! `SQLite` lexical persistence.

use std::path::{Path, PathBuf};

use kv_application::{
    CollectionStoreError, IndexStatus, IndexStoreError, LexicalIndexStore, LexicalSearchStore,
    Position, SearchResult, SearchResultSet, SearchScope, SearchStoreError, SemanticStatus,
};
use kv_domain::{CollectionName, EmbeddingModel, FileId, PassageKind, Timestamp};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    INDEX_SCHEMA_VERSION, LEGACY_EMBEDDING_DIMENSION, database_unavailable, index_storage_failure,
    schema_version, search_storage_failure,
};

/// Reads lexical index status from an existing `SQLite` database.
pub struct SqliteLexicalIndexStore {
    pub(super) connection: Connection,
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
                    s.built_at AS built_at,
                    es.model AS semantic_model,
                    es.dimension AS semantic_dimension
             FROM collections c
             LEFT JOIN files f ON f.collection_id = c.collection_id
             LEFT JOIN lexical_index_state s ON s.collection_id = c.collection_id
             LEFT JOIN semantic_index_state es ON es.collection_id = c.collection_id
             GROUP BY c.collection_id
             ORDER BY c.name_key"
        } else {
            "SELECT c.display_name, COUNT(f.file_id) AS file_count,
                    0 AS passage_count, NULL AS built_at,
                    NULL AS semantic_model, NULL AS semantic_dimension
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
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            })
            .map_err(index_storage_failure)?;

        let mut statuses = Vec::new();
        for row in rows {
            let (
                display_name,
                file_count,
                passage_count,
                built_at,
                semantic_model,
                semantic_dimension,
            ) = row.map_err(index_storage_failure)?;
            let collection =
                CollectionName::try_from(display_name.as_str()).map_err(index_storage_failure)?;
            let semantic = match semantic_model {
                Some(model) => {
                    let model = EmbeddingModel::try_new(&model).map_err(index_storage_failure)?;
                    let dimension =
                        usize::try_from(semantic_dimension.unwrap_or(LEGACY_EMBEDDING_DIMENSION))
                            .map_err(index_storage_failure)?;
                    Some(SemanticStatus::new(model, dimension))
                }
                None => None,
            };
            statuses.push(IndexStatus::new(
                collection,
                usize::try_from(file_count).map_err(index_storage_failure)?,
                usize::try_from(passage_count).map_err(index_storage_failure)?,
                built_at
                    .and_then(|value| u64::try_from(value).ok())
                    .map(Timestamp::from_unix_seconds),
                semantic,
            ));
        }

        Ok(statuses)
    }
}

impl SqliteLexicalIndexStore {
    fn has_index_tables(&self) -> Result<bool, IndexStoreError> {
        schema_version(&self.connection)
            .map_err(index_storage_failure)
            .map(|version| version >= INDEX_SCHEMA_VERSION)
    }
}

/// Searches the lexical index of an existing `SQLite` database.
pub struct SqliteLexicalSearchStore {
    pub(super) connection: Connection,
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

        let version = schema_version(&self.connection).map_err(search_storage_failure)?;
        let has_index = version >= INDEX_SCHEMA_VERSION;
        if !has_index {
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

        let results = self.search_results(query, collection_id, limit, version >= 4)?;
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
        has_offsets: bool,
    ) -> Result<Vec<SearchResult>, SearchStoreError> {
        let offset_expression = if has_offsets {
            "pf.byte_offset"
        } else {
            "NULL"
        };
        let sql = format!(
            "SELECT c.display_name, f.path, pf.kind, passages.content,
                    {offset_expression} AS byte_offset,
                    f.content AS file_content,
                    bm25(passages) AS rank, f.file_id
             FROM passages
             JOIN passage_files pf ON pf.passage_rowid = passages.rowid
             JOIN files f ON f.file_id = pf.file_id
             JOIN collections c ON c.collection_id = pf.collection_id
             JOIN lexical_index_state s ON s.collection_id = c.collection_id
             WHERE passages MATCH ?1 AND (?2 IS NULL OR c.collection_id = ?2)
             ORDER BY rank ASC, c.name_key, f.path, pf.position
             LIMIT ?3"
        );

        let mut statement = self
            .connection
            .prepare(&sql)
            .map_err(search_storage_failure)?;
        let rows = statement
            .query_map(params![query, collection_id, limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, f64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(search_query_failure)?;

        let mut results = Vec::new();
        for row in rows {
            let (display_name, path, kind, text, byte_offset, file_content, rank, file_id) =
                row.map_err(search_query_failure)?;
            let collection =
                CollectionName::try_from(display_name.as_str()).map_err(search_storage_failure)?;
            let kind = PassageKind::from_key(&kind).ok_or_else(|| {
                SearchStoreError::Storage(Box::new(std::io::Error::other("unknown passage kind")))
            })?;
            let position = compute_position(byte_offset, text.len(), &file_content);
            results.push(SearchResult::new(
                kv_application::SearchFile::new(
                    FileId::try_new(u64::try_from(file_id).map_err(search_storage_failure)?)
                        .map_err(search_storage_failure)?,
                    collection,
                    PathBuf::from(path),
                ),
                kind,
                text,
                -rank,
                position,
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

/// Maps a search execution failure to a storage error.
///
/// The query expression is produced by the domain free-text mapper and is
/// valid FTS5 by construction, so query-path execution failures are storage
/// failures. `InvalidQuery` remains in the error enum as defense-in-depth but
/// is never constructed from engine error text (ADR-009).
pub(super) fn search_query_failure(error: rusqlite::Error) -> SearchStoreError {
    SearchStoreError::Storage(Box::new(error))
}

/// Computes a passage's position from its stored byte offset and file content.
///
/// When the offset is unknown (a database not yet migrated to schema version
/// 4), the line range is reported as unknown (0) while the byte length is kept.
pub(crate) fn compute_position(
    byte_offset: Option<i64>,
    byte_length: usize,
    content: &[u8],
) -> Position {
    let Some(offset) = byte_offset.and_then(|value| usize::try_from(value).ok()) else {
        return Position::new(0, byte_length, 0, 0);
    };
    let line_start = content
        .iter()
        .take(offset)
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1;
    let line_end = content
        .iter()
        .take(offset.saturating_add(byte_length))
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1;
    Position::new(offset, byte_length, line_start, line_end)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use rusqlite::Connection;

    use crate::register_vector_extension;

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
