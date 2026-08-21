use std::path::Path;

use kv_application::{
    CollectionStoreError, HybridCandidate, HybridCandidates, HybridSearchStore,
    HybridSearchStoreError, SearchScope,
};
use kv_domain::{
    CollectionName, ContentHash, Embedding, EmbeddingModel, FileId, PassageKey, PassageKind,
    RerankerModel,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{compute_position, schema_version, storage_failure, vector_blob};

/// Searches the hybrid (lexical + semantic) index of an existing `SQLite`
/// database.
pub struct SqliteHybridSearchStore {
    connection: Connection,
}

impl SqliteHybridSearchStore {
    /// Opens an existing database for hybrid search without creating or
    /// initializing it.
    ///
    /// # Errors
    ///
    /// Returns a database-not-found error when the file does not exist, or a
    /// database-unavailable error when it cannot be opened.
    pub fn open(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }

        let connection = Connection::open(path).map_err(storage_failure)?;
        crate::register_vector_extension(&connection).map_err(storage_failure)?;

        Ok(Self { connection })
    }
}

impl HybridSearchStore for SqliteHybridSearchStore {
    fn global_model(&self) -> Result<Option<EmbeddingModel>, HybridSearchStoreError> {
        let value = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'embed_model' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(hybrid_storage_failure)?;

        value
            .map(|value| {
                EmbeddingModel::try_new(&value)
                    .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))
            })
            .transpose()
    }

    fn reranker_model(&self) -> Result<Option<RerankerModel>, HybridSearchStoreError> {
        let value = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'reranker_model' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(hybrid_storage_failure)?;

        value
            .map(|value| {
                RerankerModel::try_new(&value)
                    .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))
            })
            .transpose()
    }

    fn candidates(
        &self,
        fts5_query: &str,
        query_embedding: Option<&Embedding>,
        scope: SearchScope<'_>,
        pool: usize,
    ) -> Result<HybridCandidates, HybridSearchStoreError> {
        let version = schema_version(&self.connection).map_err(hybrid_storage_failure)?;
        if version < 3 {
            return match scope {
                SearchScope::All => Ok(HybridCandidates::new(Vec::new(), Vec::new())),
                SearchScope::Collection(_) => Err(HybridSearchStoreError::IndexNotBuilt),
            };
        }

        let collection_id = self.resolve_scope(scope)?;
        self.check_staleness(collection_id)?;
        self.check_dimensions(collection_id)?;

        let lexical = self.lexical_leg(fts5_query, collection_id, pool)?;
        let semantic = match query_embedding {
            Some(embedding) => self.semantic_leg(embedding, collection_id, pool)?,
            None => Vec::new(),
        };

        Ok(HybridCandidates::new(lexical, semantic))
    }
}

impl SqliteHybridSearchStore {
    /// Resolves the scope to an optional collection ID, validating existence and
    /// lexical-index build state for a targeted collection.
    fn resolve_scope(&self, scope: SearchScope<'_>) -> Result<Option<i64>, HybridSearchStoreError> {
        match scope {
            SearchScope::All => Ok(None),
            SearchScope::Collection(collection) => {
                let collection_id = self
                    .resolve_collection_id(collection)
                    .map_err(hybrid_storage_failure)?
                    .ok_or(HybridSearchStoreError::CollectionNotFound)?;
                let built = self
                    .index_is_built(collection_id)
                    .map_err(hybrid_storage_failure)?;
                if !built {
                    return Err(HybridSearchStoreError::IndexNotBuilt);
                }
                Ok(Some(collection_id))
            }
        }
    }

    /// Fails when any in-scope collection's semantic index is stale.
    ///
    /// A collection is stale when its stored file-set fingerprint differs from
    /// the current `files` fingerprint for the same collection.
    fn check_staleness(&self, collection_id: Option<i64>) -> Result<(), HybridSearchStoreError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT s.collection_id, s.file_set_fingerprint
                 FROM semantic_index_state s
                 JOIN collections c ON c.collection_id = s.collection_id
                 WHERE ?1 IS NULL OR c.collection_id = ?1
                 ORDER BY c.name_key",
            )
            .map_err(hybrid_storage_failure)?;
        let rows = statement
            .query_map(params![collection_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(hybrid_storage_failure)?;

        for row in rows {
            let (id, stored_fingerprint) = row.map_err(hybrid_storage_failure)?;
            let current = self
                .current_fingerprint(id)
                .map_err(hybrid_storage_failure)?;
            let stored = ContentHash::try_from_hex(&stored_fingerprint)
                .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))?;
            if current != stored {
                return Err(HybridSearchStoreError::StaleSemanticIndex);
            }
        }

        Ok(())
    }

    /// Fails when any in-scope collection's recorded dimension disagrees with
    /// the active embedding dimension.
    ///
    /// A collection with no recorded dimension is legacy and read as 384.
    fn check_dimensions(&self, collection_id: Option<i64>) -> Result<(), HybridSearchStoreError> {
        let active = self.active_dimension()?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT c.display_name, COALESCE(s.dimension, ?1) AS dimension
                 FROM semantic_index_state s
                 JOIN collections c ON c.collection_id = s.collection_id
                 WHERE ?2 IS NULL OR c.collection_id = ?2
                 ORDER BY c.name_key",
            )
            .map_err(hybrid_storage_failure)?;
        let rows = statement
            .query_map(params![active, collection_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(hybrid_storage_failure)?;

        for row in rows {
            let (display_name, dimension) = row.map_err(hybrid_storage_failure)?;
            if dimension != active {
                return Err(HybridSearchStoreError::DimensionMismatch {
                    collection: display_name,
                });
            }
        }

        Ok(())
    }

    /// Returns the recorded active embedding dimension, defaulting to the
    /// legacy 384 when the setting is absent.
    fn active_dimension(&self) -> Result<i64, HybridSearchStoreError> {
        let dimension = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'embedding_dimension' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(hybrid_storage_failure)?;

        match dimension {
            Some(raw) => raw.parse::<i64>().map_err(|error| {
                HybridSearchStoreError::Storage(Box::new(std::io::Error::other(error)))
            }),
            None => Ok(384),
        }
    }

    /// Computes the current file-set fingerprint for one collection.
    fn current_fingerprint(&self, collection_id: i64) -> Result<ContentHash, rusqlite::Error> {
        let mut statement = self.connection.prepare(
            "SELECT path, content_hash FROM files WHERE collection_id = ?1 ORDER BY path",
        )?;
        let rows = statement.query_map(params![collection_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut files = Vec::new();
        for row in rows {
            let (path, hash) = row?;
            let content_hash = ContentHash::try_from_hex(&hash).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            files.push((std::path::PathBuf::from(path), content_hash));
        }
        let paths = files
            .iter()
            .map(|(path, hash)| (path.as_path(), hash))
            .collect::<Vec<_>>();
        Ok(kv_domain::file_set_fingerprint(&paths))
    }

    /// Runs the lexical (FTS5) leg, returning up to `pool` candidates.
    fn lexical_leg(
        &self,
        query: &str,
        collection_id: Option<i64>,
        pool: usize,
    ) -> Result<Vec<HybridCandidate>, HybridSearchStoreError> {
        let pool = i64::try_from(pool).map_err(hybrid_storage_failure)?;
        let sql = "SELECT c.display_name, f.path, pf.kind, passages.content,
                          pf.byte_offset, f.content, pf.file_id, pf.position,
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
            .map_err(hybrid_storage_failure)?;
        let rows = statement
            .query_map(params![query, collection_id, pool], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, f64>(8)?,
                ))
            })
            .map_err(hybrid_storage_failure)?;

        let mut candidates = Vec::new();
        for row in rows {
            let (
                display_name,
                path,
                kind,
                text,
                byte_offset,
                file_content,
                file_id,
                position,
                rank,
            ) = row.map_err(hybrid_storage_failure)?;
            candidates.push(hybrid_candidate(
                &display_name,
                &path,
                &kind,
                &text,
                byte_offset,
                &file_content,
                file_id,
                position,
                -rank,
            )?);
        }

        Ok(candidates)
    }

    /// Runs the semantic (`knn_match`) leg, returning up to `pool` candidates.
    fn semantic_leg(
        &self,
        query_embedding: &Embedding,
        collection_id: Option<i64>,
        pool: usize,
    ) -> Result<Vec<HybridCandidate>, HybridSearchStoreError> {
        let table_exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = 'embeddings'
                )",
                [],
                |row| row.get(0),
            )
            .map_err(hybrid_storage_failure)?;
        if !table_exists {
            return Ok(Vec::new());
        }

        let blob = vector_blob(query_embedding.as_slice());
        let pool = i64::try_from(pool).map_err(hybrid_storage_failure)?;
        let sql = "SELECT c.display_name, f.path, e.kind, passages.content,
                          pf.byte_offset, f.content, e.file_id, e.position,
                          e.distance
                   FROM embeddings e
                   JOIN passage_files pf ON pf.collection_id = e.collection_id
                        AND pf.file_id = e.file_id
                        AND pf.kind = e.kind
                        AND pf.position = e.position
                   JOIN passages ON passages.rowid = pf.passage_rowid
                   JOIN files f ON f.file_id = e.file_id
                   JOIN collections c ON c.collection_id = e.collection_id
                   WHERE knn_match(e.distance, ?1)
                         AND (?2 IS NULL OR e.collection_id = ?2)
                   ORDER BY e.distance ASC
                   LIMIT ?3";

        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(hybrid_storage_failure)?;
        let rows = statement
            .query_map(params![blob, collection_id, pool], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, f64>(8)?,
                ))
            })
            .map_err(hybrid_storage_failure)?;

        let mut candidates = Vec::new();
        for row in rows {
            let (
                display_name,
                path,
                kind,
                text,
                byte_offset,
                file_content,
                file_id,
                position,
                distance,
            ) = row.map_err(hybrid_storage_failure)?;
            let similarity = 1.0 - distance;
            candidates.push(hybrid_candidate(
                &display_name,
                &path,
                &kind,
                &text,
                byte_offset,
                &file_content,
                file_id,
                position,
                similarity,
            )?);
        }

        Ok(candidates)
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

/// Builds a hybrid candidate from a retrieved row.
#[allow(clippy::too_many_arguments)]
fn hybrid_candidate(
    display_name: &str,
    path: &str,
    kind: &str,
    text: &str,
    byte_offset: Option<i64>,
    file_content: &[u8],
    file_id: i64,
    position: i64,
    score: f64,
) -> Result<HybridCandidate, HybridSearchStoreError> {
    let collection = CollectionName::try_from(display_name)
        .map_err(|error| HybridSearchStoreError::Storage(Box::new(error)))?;
    let kind = PassageKind::from_key(kind).ok_or_else(|| {
        HybridSearchStoreError::Storage(Box::new(std::io::Error::other("unknown passage kind")))
    })?;
    let file_id = u64::try_from(file_id).map_err(hybrid_storage_failure)?;
    let file = FileId::try_new(file_id).map_err(hybrid_storage_failure)?;
    let position = usize::try_from(position).map_err(hybrid_storage_failure)?;
    let position_value = compute_position(byte_offset, text.len(), file_content);

    Ok(HybridCandidate::new(
        PassageKey::new(file, kind, position),
        collection,
        std::path::PathBuf::from(path),
        kind,
        text.to_owned(),
        score,
        position_value,
    ))
}

/// Reads a global model setting by key.
fn hybrid_storage_failure(
    error: impl std::error::Error + Send + Sync + 'static,
) -> HybridSearchStoreError {
    HybridSearchStoreError::Storage(Box::new(error))
}
