//! `SQLite` semantic commit persistence.

use kv_application::SemanticIndexStoreError;
use kv_domain::{ContentHash, Embedding, EmbeddingModel, RerankerModel, SemanticPassage};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::{
    fingerprint_for_collection, rebuild_graph, rebuild_index, semantic_storage_failure,
    upsert_file, vector_blob,
};

pub(super) fn reconcile_files_and_indexes(
    transaction: &Transaction<'_>,
    update: &kv_application::CollectionIndexUpdate,
    collection_id: i64,
    at: i64,
) -> Result<(), SemanticIndexStoreError> {
    for file in &update.upsert {
        upsert_file(transaction, collection_id, file, at).map_err(semantic_storage_failure)?;
    }
    for path in &update.delete {
        transaction
            .execute(
                "DELETE FROM files WHERE collection_id = ?1 AND path = ?2",
                params![collection_id, path.to_string_lossy().as_ref()],
            )
            .map_err(semantic_storage_failure)?;
    }
    rebuild_index(transaction, collection_id, at).map_err(semantic_storage_failure)?;
    rebuild_graph(transaction, collection_id, at).map_err(semantic_storage_failure)
}

pub(super) fn reconcile_semantic_update(
    transaction: &Transaction<'_>,
    update: &kv_application::CollectionIndexUpdate,
    collection_id: i64,
    active_dimension: i64,
    at: i64,
) -> Result<(), SemanticIndexStoreError> {
    let enabled: bool = transaction
        .query_row(
            "SELECT semantic_enabled FROM collections WHERE collection_id = ?1",
            params![collection_id],
            |row| row.get::<_, i64>(0).map(|value| value != 0),
        )
        .map_err(semantic_storage_failure)?;
    if !enabled {
        return clear_semantic_collection(transaction, collection_id);
    }
    let table_exists = embeddings_table_exists(transaction)?;
    let dimension = semantic_dimension(update, active_dimension)?;
    ensure_semantic_dimension(transaction, table_exists, dimension, active_dimension)?;
    ensure_configured_model(transaction, update)?;
    if table_exists {
        transaction
            .execute(
                "DELETE FROM embeddings WHERE collection_id = ?1",
                params![collection_id],
            )
            .map_err(semantic_storage_failure)?;
    }
    insert_prepared_semantic(transaction, update, collection_id, dimension)?;
    update_semantic_index_state(transaction, update, collection_id, dimension, at)
}

pub(super) fn embeddings_table_exists(
    transaction: &Transaction<'_>,
) -> Result<bool, SemanticIndexStoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'embeddings')",
            [],
            |row| row.get(0),
        )
        .map_err(semantic_storage_failure)
}

pub(super) fn clear_semantic_collection(
    transaction: &Transaction<'_>,
    collection_id: i64,
) -> Result<(), SemanticIndexStoreError> {
    if embeddings_table_exists(transaction)? {
        transaction
            .execute(
                "DELETE FROM embeddings WHERE collection_id = ?1",
                params![collection_id],
            )
            .map_err(semantic_storage_failure)?;
    }
    transaction
        .execute(
            "DELETE FROM semantic_index_state WHERE collection_id = ?1",
            params![collection_id],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn semantic_dimension(
    update: &kv_application::CollectionIndexUpdate,
    active_dimension: i64,
) -> Result<i64, SemanticIndexStoreError> {
    update
        .semantic
        .first()
        .map(|passage| i64::try_from(passage.embedding().len()))
        .transpose()
        .map_err(semantic_storage_failure)
        .map(|dimension| dimension.unwrap_or(active_dimension))
}

pub(super) fn ensure_semantic_dimension(
    transaction: &Transaction<'_>,
    table_exists: bool,
    dimension: i64,
    active_dimension: i64,
) -> Result<(), SemanticIndexStoreError> {
    if table_exists && dimension != active_dimension {
        return Err(semantic_storage_message(
            "collection update embedding dimension differs from the active model",
        ));
    }
    if !table_exists {
        transaction
            .execute_batch(&format!(
                "CREATE VIRTUAL TABLE embeddings USING vector(
                    dim={dimension}, type=float4, metric=cosine,
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
    }
    Ok(())
}

pub(super) fn ensure_configured_model(
    transaction: &Transaction<'_>,
    update: &kv_application::CollectionIndexUpdate,
) -> Result<(), SemanticIndexStoreError> {
    let configured: Option<String> = transaction
        .query_row(
            "SELECT value FROM settings WHERE key = 'embed_model'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(semantic_storage_failure)?;
    if configured
        .as_deref()
        .is_some_and(|configured| configured != update.model.as_str())
    {
        return Err(semantic_storage_message(
            "staged vectors do not match the configured embedding model",
        ));
    }
    transaction
        .execute(
            "INSERT INTO settings(key, value) VALUES ('embed_model', ?1)
             ON CONFLICT(key) DO NOTHING",
            params![update.model.as_str()],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn insert_prepared_semantic(
    transaction: &Transaction<'_>,
    update: &kv_application::CollectionIndexUpdate,
    collection_id: i64,
    dimension: i64,
) -> Result<(), SemanticIndexStoreError> {
    for staged in &update.semantic {
        if i64::try_from(staged.embedding().len()).map_err(semantic_storage_failure)? != dimension {
            return Err(semantic_storage_message(
                "staged embedding dimensions do not match",
            ));
        }
        let path = staged.path().to_string_lossy();
        let file_id: i64 = transaction
            .query_row(
                "SELECT file_id FROM files WHERE collection_id = ?1 AND path = ?2",
                params![collection_id, path.as_ref()],
                |row| row.get(0),
            )
            .map_err(semantic_storage_failure)?;
        transaction
            .execute(
                "INSERT INTO embeddings(vector, collection_id, file_id, kind, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    vector_blob(staged.embedding().as_slice()),
                    collection_id,
                    file_id,
                    staged.kind().as_str(),
                    i64::try_from(staged.position()).map_err(semantic_storage_failure)?,
                ],
            )
            .map_err(semantic_storage_failure)?;
    }
    Ok(())
}

pub(super) fn update_semantic_index_state(
    transaction: &Transaction<'_>,
    update: &kv_application::CollectionIndexUpdate,
    collection_id: i64,
    dimension: i64,
    at: i64,
) -> Result<(), SemanticIndexStoreError> {
    let fingerprint = fingerprint_for_collection(transaction, collection_id)?;
    transaction
        .execute(
            "INSERT INTO semantic_index_state(
                collection_id, file_set_fingerprint, model, dimension, passage_count, embedded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(collection_id) DO UPDATE SET
                file_set_fingerprint=excluded.file_set_fingerprint,
                model=excluded.model, dimension=excluded.dimension,
                passage_count=excluded.passage_count, embedded_at=excluded.embedded_at",
            params![
                collection_id,
                fingerprint.as_str(),
                update.model.as_str(),
                dimension,
                i64::try_from(update.semantic.len()).map_err(semantic_storage_failure)?,
                at,
            ],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn semantic_storage_message(message: &'static str) -> SemanticIndexStoreError {
    SemanticIndexStoreError::Storage(Box::new(std::io::Error::other(message)))
}

pub(super) fn configure_global_vector_table(
    transaction: &Transaction<'_>,
    table_exists: bool,
    current_dimension: i64,
    next_dimension: i64,
) -> Result<(), SemanticIndexStoreError> {
    if !table_exists || current_dimension != next_dimension {
        transaction
            .execute_batch(&format!(
                "DROP TABLE IF EXISTS embeddings;
                 CREATE VIRTUAL TABLE embeddings USING vector(
                     dim={next_dimension}, type=float4, metric=cosine,
                     metadata=\"collection_id INTEGER, file_id INTEGER, kind TEXT, position INTEGER\"
                 );"
            ))
            .map_err(semantic_storage_failure)?;
    }
    transaction
        .execute(
            "INSERT INTO settings(key, value) VALUES ('embedding_dimension', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![next_dimension],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn store_global_models(
    transaction: &Transaction<'_>,
    model: &EmbeddingModel,
    reranker: Option<&RerankerModel>,
) -> Result<(), SemanticIndexStoreError> {
    if let Some(reranker) = reranker {
        transaction
            .execute(
                "INSERT INTO settings(key, value) VALUES ('reranker_model', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![reranker.as_str()],
            )
            .map_err(semantic_storage_failure)?;
    }
    transaction
        .execute(
            "INSERT INTO settings(key, value) VALUES ('embed_model', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![model.as_str()],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn replace_collection_semantic(
    transaction: &Transaction<'_>,
    collection_id: i64,
    fingerprint: &ContentHash,
    model: &EmbeddingModel,
    dimension: i64,
    embedded_at: i64,
    embeddings: &[(SemanticPassage, Embedding)],
) -> Result<(), SemanticIndexStoreError> {
    transaction
        .execute(
            "DELETE FROM embeddings WHERE collection_id = ?1",
            params![collection_id],
        )
        .map_err(semantic_storage_failure)?;
    insert_collection_embeddings(transaction, collection_id, dimension, embeddings)?;
    let passage_count = i64::try_from(embeddings.len()).map_err(semantic_storage_failure)?;
    transaction
        .execute(
            "INSERT INTO semantic_index_state(
                collection_id, file_set_fingerprint, model, dimension, passage_count, embedded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(collection_id) DO UPDATE SET
                file_set_fingerprint=excluded.file_set_fingerprint,
                model=excluded.model, dimension=excluded.dimension,
                passage_count=excluded.passage_count, embedded_at=excluded.embedded_at",
            params![
                collection_id,
                fingerprint.as_str(),
                model.as_str(),
                dimension,
                passage_count,
                embedded_at,
            ],
        )
        .map_err(semantic_storage_failure)?;
    Ok(())
}

pub(super) fn insert_collection_embeddings(
    transaction: &Transaction<'_>,
    collection_id: i64,
    dimension: i64,
    embeddings: &[(SemanticPassage, Embedding)],
) -> Result<(), SemanticIndexStoreError> {
    for (passage, embedding) in embeddings {
        if i64::try_from(embedding.len()).map_err(semantic_storage_failure)? != dimension {
            return Err(semantic_storage_message(
                "staged embedding dimensions do not match",
            ));
        }
        let vector = vector_blob(embedding.as_slice());
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
                    position
                ],
            )
            .map_err(semantic_storage_failure)?;
    }
    Ok(())
}
