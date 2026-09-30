//! `SQLite` semantic persistence.

use std::path::PathBuf;

use kv_application::{EmbedTarget, SemanticIndexStore, SemanticIndexStoreError};
use kv_domain::{
    CollectionName, ContentHash, Embedding, EmbeddingModel, FileId, PassageKind, RerankerModel,
    SemanticIndexStatus, SemanticPassage, Timestamp, file_set_fingerprint,
};
use rusqlite::{OptionalExtension, params};

use crate::{
    INDEX_SCHEMA_VERSION, LEGACY_EMBEDDING_DIMENSION, SqliteFileStore,
    configure_global_vector_table, reconcile_files_and_indexes, reconcile_semantic_update,
    replace_collection_semantic, resolve_collection_id, schema_version, semantic_storage_failure,
    store_global_models,
};

/// Semantic-index view of the same connection used by [`SqliteFileStore`].
pub type SqliteSemanticIndexStore = SqliteFileStore;

impl SemanticIndexStore for SqliteSemanticIndexStore {
    fn semantic_enabled(
        &self,
        collection: &CollectionName,
    ) -> Result<bool, SemanticIndexStoreError> {
        let enabled = self
            .connection
            .query_row(
                "SELECT semantic_enabled FROM collections WHERE name_key = ?1",
                params![collection.name_key()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;
        Ok(enabled != 0)
    }

    fn set_semantic_enabled(
        &mut self,
        collection: &CollectionName,
        enabled: bool,
    ) -> Result<(), SemanticIndexStoreError> {
        let changed = self
            .connection
            .execute(
                "UPDATE collections SET semantic_enabled = ?1 WHERE name_key = ?2",
                params![i64::from(enabled), collection.name_key()],
            )
            .map_err(semantic_storage_failure)?;
        if changed == 0 {
            return Err(SemanticIndexStoreError::CollectionNotFound);
        }
        Ok(())
    }

    fn reconcile_collection(
        &mut self,
        update: &kv_application::CollectionIndexUpdate,
        at: Timestamp,
    ) -> Result<(), SemanticIndexStoreError> {
        let at = i64::try_from(at.as_unix_seconds()).map_err(semantic_storage_failure)?;
        let active_dimension = self.active_dimension()?;
        let transaction = self
            .connection
            .transaction()
            .map_err(semantic_storage_failure)?;
        let collection_id = resolve_collection_id(&transaction, &update.collection)
            .map_err(semantic_storage_failure)?;
        reconcile_files_and_indexes(&transaction, update, collection_id, at)?;
        reconcile_semantic_update(&transaction, update, collection_id, active_dimension, at)?;
        transaction.commit().map_err(semantic_storage_failure)
    }

    fn targets(&self) -> Result<Vec<EmbedTarget>, SemanticIndexStoreError> {
        let has_index = schema_version(&self.connection).map_err(semantic_storage_failure)?
            >= INDEX_SCHEMA_VERSION;

        let sql = if has_index {
            "SELECT c.display_name,
                    EXISTS(SELECT 1 FROM files f WHERE f.collection_id = c.collection_id),
                    EXISTS(SELECT 1 FROM lexical_index_state s WHERE s.collection_id = c.collection_id)
             FROM collections c
             ORDER BY c.name_key"
        } else {
            "SELECT c.display_name,
                    EXISTS(SELECT 1 FROM files f WHERE f.collection_id = c.collection_id),
                    0
             FROM collections c
             ORDER BY c.name_key"
        };

        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(semantic_storage_failure)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(semantic_storage_failure)?;

        let mut targets = Vec::new();
        for row in rows {
            let (display_name, has_files, lexical_built) = row.map_err(semantic_storage_failure)?;
            let collection = CollectionName::try_from(display_name.as_str())
                .map_err(semantic_storage_failure)?;
            targets.push(EmbedTarget::new(
                collection,
                has_files != 0,
                lexical_built != 0,
            ));
        }

        Ok(targets)
    }

    fn resolve(&self, collection: &CollectionName) -> Result<EmbedTarget, SemanticIndexStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;

        let has_index = schema_version(&self.connection).map_err(semantic_storage_failure)?
            >= INDEX_SCHEMA_VERSION;

        let file_count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM files WHERE collection_id = ?1",
                params![collection_id],
                |row| row.get(0),
            )
            .map_err(semantic_storage_failure)?;

        let lexical_built = if has_index {
            self.connection
                .query_row(
                    "SELECT 1 FROM lexical_index_state WHERE collection_id = ?1 LIMIT 1",
                    params![collection_id],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(semantic_storage_failure)?
                .is_some()
        } else {
            false
        };

        Ok(EmbedTarget::new(
            collection.clone(),
            file_count > 0,
            lexical_built,
        ))
    }

    fn global_model(&self) -> Result<Option<EmbeddingModel>, SemanticIndexStoreError> {
        let model = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'embed_model' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(semantic_storage_failure)?;

        model
            .map(|value| EmbeddingModel::try_new(&value).map_err(semantic_storage_failure))
            .transpose()
    }

    fn set_global_model(&mut self, model: &EmbeddingModel) -> Result<(), SemanticIndexStoreError> {
        self.connection
            .execute(
                "INSERT INTO settings(key, value) VALUES ('embed_model', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![model.as_str()],
            )
            .map_err(semantic_storage_failure)?;

        Ok(())
    }

    fn reranker_model(&self) -> Result<Option<RerankerModel>, SemanticIndexStoreError> {
        let model = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'reranker_model' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(semantic_storage_failure)?;

        model
            .map(|value| RerankerModel::try_new(&value).map_err(semantic_storage_failure))
            .transpose()
    }

    fn set_reranker_model(&mut self, model: &RerankerModel) -> Result<(), SemanticIndexStoreError> {
        self.connection
            .execute(
                "INSERT INTO settings(key, value) VALUES ('reranker_model', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![model.as_str()],
            )
            .map_err(semantic_storage_failure)?;

        Ok(())
    }

    fn ensure_dimension(&mut self, dimension: usize) -> Result<(), SemanticIndexStoreError> {
        let dimension = i64::try_from(dimension).map_err(semantic_storage_failure)?;
        let active = self.active_dimension()?;
        let table_exists = self.embeddings_table_exists()?;

        if table_exists && active == dimension {
            return Ok(());
        }

        let transaction = self
            .connection
            .transaction()
            .map_err(semantic_storage_failure)?;
        transaction
            .execute_batch(&format!(
                "DROP TABLE IF EXISTS embeddings;
                 CREATE VIRTUAL TABLE embeddings USING vector(
                     dim={dimension},
                     type=float4,
                     metric=cosine,
                     metadata=\"collection_id INTEGER, file_id INTEGER, kind TEXT, position INTEGER\"
                 );"
            ))
            .map_err(semantic_storage_failure)?;
        transaction
            .execute(
                "INSERT INTO settings(key, value) VALUES ('embedding_dimension', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![dimension],
            )
            .map_err(semantic_storage_failure)?;
        transaction.commit().map_err(semantic_storage_failure)?;

        Ok(())
    }

    fn status(
        &self,
        collection: &CollectionName,
    ) -> Result<Option<SemanticIndexStatus>, SemanticIndexStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;

        let row = self
            .connection
            .query_row(
                "SELECT file_set_fingerprint, model, dimension, passage_count, embedded_at
                 FROM semantic_index_state
                 WHERE collection_id = ?1 LIMIT 1",
                params![collection_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(semantic_storage_failure)?;

        row.map(
            |(fingerprint, model, dimension, passage_count, embedded_at)| {
                Ok(SemanticIndexStatus::new(
                    ContentHash::try_from_hex(&fingerprint).map_err(semantic_storage_failure)?,
                    EmbeddingModel::try_new(&model).map_err(semantic_storage_failure)?,
                    usize::try_from(dimension.unwrap_or(LEGACY_EMBEDDING_DIMENSION))
                        .map_err(semantic_storage_failure)?,
                    usize::try_from(passage_count).map_err(semantic_storage_failure)?,
                    Timestamp::from_unix_seconds(
                        u64::try_from(embedded_at).map_err(semantic_storage_failure)?,
                    ),
                ))
            },
        )
        .transpose()
    }

    fn embedded_collections(&self) -> Result<Vec<CollectionName>, SemanticIndexStoreError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT c.display_name
                 FROM semantic_index_state s
                 JOIN collections c ON c.collection_id = s.collection_id
                 ORDER BY c.name_key",
            )
            .map_err(semantic_storage_failure)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(semantic_storage_failure)?;

        let mut names = Vec::new();
        for row in rows {
            let display_name = row.map_err(semantic_storage_failure)?;
            let collection = CollectionName::try_from(display_name.as_str())
                .map_err(semantic_storage_failure)?;
            names.push(collection);
        }

        Ok(names)
    }

    fn file_set_fingerprint(
        &self,
        collection: &CollectionName,
    ) -> Result<ContentHash, SemanticIndexStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;

        let mut statement = self
            .connection
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
            let content_hash =
                ContentHash::try_from_hex(&hash).map_err(semantic_storage_failure)?;
            files.push((PathBuf::from(path), content_hash));
        }
        let paths = files
            .iter()
            .map(|(path, hash)| (path.as_path(), hash))
            .collect::<Vec<_>>();

        Ok(file_set_fingerprint(&paths))
    }

    fn passages(
        &self,
        collection: &CollectionName,
    ) -> Result<Vec<SemanticPassage>, SemanticIndexStoreError> {
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;

        let mut statement = self
            .connection
            .prepare(
                "SELECT pf.file_id, pf.kind, pf.position, passages.content
                 FROM passages
                 JOIN passage_files pf ON pf.passage_rowid = passages.rowid
                 WHERE pf.collection_id = ?1
                 ORDER BY pf.file_id, pf.position",
            )
            .map_err(semantic_storage_failure)?;
        let rows = statement
            .query_map(params![collection_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(semantic_storage_failure)?;

        let mut passages = Vec::new();
        for row in rows {
            let (file_id, kind, position, text) = row.map_err(semantic_storage_failure)?;
            let file_id = u64::try_from(file_id).map_err(semantic_storage_failure)?;
            let file = FileId::try_new(file_id).map_err(semantic_storage_failure)?;
            let kind = PassageKind::from_key(&kind).ok_or_else(|| {
                SemanticIndexStoreError::Storage(Box::new(std::io::Error::other(
                    "unknown passage kind",
                )))
            })?;
            let position = usize::try_from(position).map_err(semantic_storage_failure)?;
            passages.push(SemanticPassage::new(file, kind, position, text));
        }

        Ok(passages)
    }

    fn rebuild(
        &mut self,
        collection: &CollectionName,
        model: &EmbeddingModel,
        embedded_at: Timestamp,
        embeddings: &[(SemanticPassage, Embedding)],
    ) -> Result<usize, SemanticIndexStoreError> {
        let embedded_at =
            i64::try_from(embedded_at.as_unix_seconds()).map_err(semantic_storage_failure)?;
        let fingerprint = self.file_set_fingerprint(collection)?;
        let collection_id = self
            .resolve_collection_id(collection)
            .map_err(semantic_storage_failure)?
            .ok_or(SemanticIndexStoreError::CollectionNotFound)?;

        let active = self.active_dimension()?;
        let table_exists = self.embeddings_table_exists()?;
        let transaction = self
            .connection
            .transaction()
            .map_err(semantic_storage_failure)?;
        if !table_exists {
            transaction
                .execute_batch(&format!(
                    "CREATE VIRTUAL TABLE embeddings USING vector(
                         dim={active},
                         type=float4,
                         metric=cosine,
                         metadata=\"collection_id INTEGER, file_id INTEGER, kind TEXT, position INTEGER\"
                     );"
                ))
                .map_err(semantic_storage_failure)?;
        }

        transaction
            .execute(
                "DELETE FROM embeddings WHERE collection_id = ?1",
                params![collection_id],
            )
            .map_err(semantic_storage_failure)?;

        insert_single_collection_embeddings(&transaction, collection_id, active, embeddings)?;

        let passage_count = i64::try_from(embeddings.len()).map_err(semantic_storage_failure)?;
        transaction
            .execute(
                "INSERT INTO semantic_index_state(
                    collection_id, file_set_fingerprint, model, dimension, passage_count, embedded_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(collection_id) DO UPDATE SET
                     file_set_fingerprint = excluded.file_set_fingerprint,
                     model = excluded.model,
                     dimension = excluded.dimension,
                     passage_count = excluded.passage_count,
                     embedded_at = excluded.embedded_at",
                params![
                    collection_id,
                    fingerprint.as_str(),
                    model.as_str(),
                    active,
                    passage_count,
                    embedded_at,
                ],
            )
            .map_err(semantic_storage_failure)?;

        transaction.commit().map_err(semantic_storage_failure)?;

        Ok(embeddings.len())
    }

    fn rebuild_all(
        &mut self,
        model: &EmbeddingModel,
        reranker: Option<&RerankerModel>,
        embedded_at: Timestamp,
        collections: &[kv_application::PreparedSemanticCollection],
    ) -> Result<(), SemanticIndexStoreError> {
        let embedded_at =
            i64::try_from(embedded_at.as_unix_seconds()).map_err(semantic_storage_failure)?;
        let dimension = collections
            .iter()
            .flat_map(|collection| collection.embeddings.iter())
            .next()
            .map(|(_, embedding)| embedding.len());
        let active = match dimension {
            Some(value) => i64::try_from(value).map_err(semantic_storage_failure)?,
            None => self.active_dimension()?,
        };
        let mut resolved = Vec::with_capacity(collections.len());
        for batch in collections {
            let collection_id = self
                .resolve_collection_id(&batch.collection)
                .map_err(semantic_storage_failure)?
                .ok_or(SemanticIndexStoreError::CollectionNotFound)?;
            let fingerprint = self.file_set_fingerprint(&batch.collection)?;
            resolved.push((collection_id, fingerprint, &batch.embeddings));
        }
        let table_exists = self.embeddings_table_exists()?;
        let current = self.active_dimension()?;
        let transaction = self
            .connection
            .transaction()
            .map_err(semantic_storage_failure)?;
        configure_global_vector_table(&transaction, table_exists, current, active)?;
        store_global_models(&transaction, model, reranker)?;
        for (collection_id, fingerprint, embeddings) in resolved {
            replace_collection_semantic(
                &transaction,
                collection_id,
                &fingerprint,
                model,
                active,
                embedded_at,
                embeddings,
            )?;
        }
        transaction.commit().map_err(semantic_storage_failure)
    }
}

impl SqliteSemanticIndexStore {
    /// Returns the recorded active embedding dimension, defaulting to the
    /// legacy 384 when the setting is absent.
    fn active_dimension(&self) -> Result<i64, SemanticIndexStoreError> {
        let dimension = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'embedding_dimension' LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(semantic_storage_failure)?;

        match dimension {
            Some(raw) => raw
                .parse::<i64>()
                .map_err(|error| semantic_storage_failure(std::io::Error::other(error))),
            None => Ok(LEGACY_EMBEDDING_DIMENSION),
        }
    }

    /// Returns whether the `embeddings` vector table exists.
    fn embeddings_table_exists(&self) -> Result<bool, SemanticIndexStoreError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master
                    WHERE type = 'table' AND name = 'embeddings'
                )",
                [],
                |row| row.get(0),
            )
            .map_err(semantic_storage_failure)
    }

    pub(super) fn resolve_collection_id(
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
}

/// Encodes a vector into the sqlite-vector float4 blob format.
pub(crate) fn vector_blob(values: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(values.len() * 4);
    for value in values {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

fn insert_single_collection_embeddings(
    transaction: &rusqlite::Transaction<'_>,
    collection_id: i64,
    active: i64,
    embeddings: &[(SemanticPassage, Embedding)],
) -> Result<(), SemanticIndexStoreError> {
    for (passage, embedding) in embeddings {
        let values = embedding.as_slice();
        if i64::try_from(values.len()).map_err(semantic_storage_failure)? != active {
            return Err(SemanticIndexStoreError::Storage(Box::new(
                std::io::Error::other(format!(
                    "embedding dimension mismatch: expected {active}, got {}",
                    values.len()
                )),
            )));
        }
        let vector = vector_blob(values);
        let file_id = i64::try_from(passage.file().as_u64()).map_err(semantic_storage_failure)?;
        let position = i64::try_from(passage.position()).map_err(semantic_storage_failure)?;
        transaction
            .execute(
                "INSERT INTO embeddings(vector, collection_id, file_id, kind, position)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    vector,
                    collection_id,
                    file_id,
                    passage.kind().as_str(),
                    position,
                ],
            )
            .map_err(semantic_storage_failure)?;
    }

    Ok(())
}
