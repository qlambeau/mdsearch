//! Snapshot-based index inspection (REQ-023, ADR-018).

use crate::{database_unavailable, index_storage_failure, schema_version};
use kv_application::CollectionStoreError;
use kv_application::{
    CollectionIndexInspection, DatabaseIndexInspection, IndexInspection, IndexInspectionStore,
    IndexStoreError,
};
use kv_domain::{
    CollectionName, ContentHash, EmbeddingModel, IndexEnablement, IndexFreshness, RerankerModel,
    Timestamp, file_set_fingerprint, segment_passages,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

/// Reads consistent index state without migrations or source filesystem access.
pub struct SqliteIndexInspectionStore {
    connection: Connection,
}

impl SqliteIndexInspectionStore {
    /// Opens an existing database for read-only inspection.
    ///
    /// # Errors
    /// Returns database-not-found or database-unavailable without creating files.
    pub fn open(path: &Path) -> Result<Self, CollectionStoreError> {
        if !path.exists() {
            return Err(CollectionStoreError::DatabaseNotFound);
        }
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(database_unavailable)?;
        Ok(Self { connection })
    }
}

impl IndexInspectionStore for SqliteIndexInspectionStore {
    fn inspect(
        &self,
        collection: Option<&CollectionName>,
    ) -> Result<DatabaseIndexInspection, IndexStoreError> {
        let snapshot = self
            .connection
            .unchecked_transaction()
            .map_err(index_storage_failure)?;
        let version = schema_version(&snapshot).map_err(index_storage_failure)?;
        let mut report = read_models(&snapshot, version)?;
        let sql = "SELECT collection_id, display_name FROM collections WHERE (?1 IS NULL OR name_key = ?1) ORDER BY name_key";
        let mut statement = snapshot.prepare(sql).map_err(index_storage_failure)?;
        let rows = statement
            .query_map([collection.map(CollectionName::name_key)], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(index_storage_failure)?;
        for row in rows {
            let (id, name) = row.map_err(index_storage_failure)?;
            report
                .collections
                .push(read_collection(&snapshot, version, id, &name, &report)?);
        }
        if collection.is_some() && report.collections.is_empty() {
            return Err(IndexStoreError::CollectionNotFound);
        }
        Ok(report)
    }
}

fn setting(
    connection: &Connection,
    version: i64,
    key: &str,
) -> Result<Option<String>, IndexStoreError> {
    if version < 5 {
        return Ok(None);
    }
    connection
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(index_storage_failure)
}

fn read_models(
    connection: &Connection,
    version: i64,
) -> Result<DatabaseIndexInspection, IndexStoreError> {
    let embedding = setting(connection, version, "embed_model")?
        .unwrap_or_else(|| "all-MiniLM-L6-v2".to_owned());
    let reranker = setting(connection, version, "reranker_model")?
        .unwrap_or_else(|| "bge-reranker-base".to_owned());
    let dimension = setting(connection, version, "embedding_dimension")?
        .map_or(Ok(384), |raw| {
            raw.parse::<usize>().map_err(index_storage_failure)
        })?;
    Ok(DatabaseIndexInspection {
        embedding_model: EmbeddingModel::try_new(&embedding).map_err(index_storage_failure)?,
        embedding_dimension: dimension,
        reranker_model: RerankerModel::try_new(&reranker).map_err(index_storage_failure)?,
        collections: Vec::new(),
    })
}

struct StoredContent {
    id: i64,
    path: PathBuf,
    bytes: Vec<u8>,
}

fn stored_content(
    connection: &Connection,
    version: i64,
    id: i64,
) -> Result<Vec<StoredContent>, IndexStoreError> {
    if version < 2 {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare(
            "SELECT file_id, path, content FROM files WHERE collection_id = ?1 ORDER BY file_id",
        )
        .map_err(index_storage_failure)?;
    let rows = statement
        .query_map([id], |row| {
            Ok(StoredContent {
                id: row.get(0)?,
                path: PathBuf::from(row.get::<_, String>(1)?),
                bytes: row.get(2)?,
            })
        })
        .map_err(index_storage_failure)?;
    rows.map(|row| row.map_err(index_storage_failure)).collect()
}

fn fingerprint(files: &[StoredContent]) -> ContentHash {
    let hashes = files
        .iter()
        .map(|file| (file.path.as_path(), ContentHash::from_content(&file.bytes)))
        .collect::<Vec<_>>();
    let refs = hashes
        .iter()
        .map(|(path, hash)| (*path, hash))
        .collect::<Vec<_>>();
    file_set_fingerprint(&refs)
}

fn unbuilt(enablement: IndexEnablement) -> IndexInspection {
    IndexInspection {
        enablement,
        built_at: None,
        freshness: None,
        compatible: true,
        passage_count: 0,
        node_count: 0,
        edge_count: 0,
        model: None,
        dimension: None,
    }
}

fn timestamp(raw: i64) -> Result<Timestamp, IndexStoreError> {
    Ok(Timestamp::from_unix_seconds(
        u64::try_from(raw).map_err(index_storage_failure)?,
    ))
}

fn count(raw: i64) -> Result<usize, IndexStoreError> {
    usize::try_from(raw).map_err(index_storage_failure)
}

fn freshness(matches: bool) -> IndexFreshness {
    if matches {
        IndexFreshness::Current
    } else {
        IndexFreshness::Stale
    }
}

fn read_collection(
    connection: &Connection,
    version: i64,
    id: i64,
    name: &str,
    models: &DatabaseIndexInspection,
) -> Result<CollectionIndexInspection, IndexStoreError> {
    let files = stored_content(connection, version, id)?;
    let hash = fingerprint(&files);
    let semantic = read_semantic(connection, version, id, &hash, models)?;
    Ok(CollectionIndexInspection {
        collection: CollectionName::try_from(name).map_err(index_storage_failure)?,
        file_count: files.len(),
        lexical: read_lexical(connection, version, id, &files)?,
        graph: read_graph(connection, version, id, &hash)?,
        semantic,
    })
}

#[derive(Eq, PartialEq)]
struct IndexedPassage {
    file: i64,
    kind: String,
    position: i64,
    offset: Option<i64>,
    text: String,
}

fn expected_passages(
    files: &[StoredContent],
    version: i64,
) -> Result<Vec<IndexedPassage>, IndexStoreError> {
    let mut expected = Vec::new();
    for file in files {
        for (position, passage) in segment_passages(&file.bytes).0.into_iter().enumerate() {
            expected.push(IndexedPassage {
                file: file.id,
                kind: passage.kind().as_str().to_owned(),
                position: i64::try_from(position).map_err(index_storage_failure)?,
                offset: if version >= 4 {
                    Some(i64::try_from(passage.byte_offset()).map_err(index_storage_failure)?)
                } else {
                    None
                },
                text: passage.text().to_owned(),
            });
        }
    }
    Ok(expected)
}

fn persisted_passages(
    connection: &Connection,
    version: i64,
    id: i64,
) -> Result<Vec<IndexedPassage>, IndexStoreError> {
    let offset = if version >= 4 {
        "pf.byte_offset"
    } else {
        "NULL"
    };
    let sql = format!(
        "SELECT pf.file_id, pf.kind, pf.position, {offset}, p.content FROM passage_files pf LEFT JOIN passages p ON p.rowid = pf.passage_rowid WHERE pf.collection_id = ?1 ORDER BY pf.file_id, pf.position, pf.passage_rowid"
    );
    let mut statement = connection.prepare(&sql).map_err(index_storage_failure)?;
    let rows = statement
        .query_map([id], |row| {
            Ok(IndexedPassage {
                file: row.get(0)?,
                kind: row.get(1)?,
                position: row.get(2)?,
                offset: row.get(3)?,
                text: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            })
        })
        .map_err(index_storage_failure)?;
    rows.map(|row| row.map_err(index_storage_failure)).collect()
}

fn read_lexical(
    connection: &Connection,
    version: i64,
    id: i64,
    files: &[StoredContent],
) -> Result<IndexInspection, IndexStoreError> {
    let mut inspection = unbuilt(IndexEnablement::Enabled);
    if version < 3 {
        return Ok(inspection);
    }
    let state = connection
        .query_row(
            "SELECT passage_count, built_at FROM lexical_index_state WHERE collection_id = ?1",
            [id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(index_storage_failure)?;
    if let Some((passage_count, built_at)) = state {
        let expected = expected_passages(files, version)?;
        let persisted = persisted_passages(connection, version, id)?;
        inspection.built_at = Some(timestamp(built_at)?);
        inspection.passage_count = count(passage_count)?;
        inspection.freshness = Some(freshness(
            expected == persisted && expected.len() == inspection.passage_count,
        ));
    }
    Ok(inspection)
}

fn read_graph(
    connection: &Connection,
    version: i64,
    id: i64,
    hash: &ContentHash,
) -> Result<IndexInspection, IndexStoreError> {
    let mut inspection = unbuilt(IndexEnablement::Enabled);
    if version < 6 {
        return Ok(inspection);
    }
    let state = connection.query_row("SELECT file_set_fingerprint, node_count, edge_count, built_at FROM graph_state WHERE collection_id = ?1", [id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))).optional().map_err(index_storage_failure)?;
    if let Some((recorded, nodes, edges, built_at)) = state {
        inspection.built_at = Some(timestamp(built_at)?);
        inspection.node_count = count(nodes)?;
        inspection.edge_count = count(edges)?;
        inspection.freshness = Some(freshness(recorded == hash.as_str()));
    }
    Ok(inspection)
}

fn read_semantic(
    connection: &Connection,
    version: i64,
    id: i64,
    hash: &ContentHash,
    models: &DatabaseIndexInspection,
) -> Result<IndexInspection, IndexStoreError> {
    let mut inspection = unbuilt(IndexEnablement::Disabled);
    if version < 5 {
        return Ok(inspection);
    }
    let dimension = if version >= 7 {
        "COALESCE(dimension, 384)"
    } else {
        "384"
    };
    let sql = format!(
        "SELECT file_set_fingerprint, model, {dimension}, passage_count, embedded_at FROM semantic_index_state WHERE collection_id = ?1"
    );
    let state = connection
        .query_row(&sql, [id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .optional()
        .map_err(index_storage_failure)?;
    let enabled = if version >= 9 {
        connection
            .query_row(
                "SELECT semantic_enabled FROM collections WHERE collection_id = ?1",
                [id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(index_storage_failure)?
    } else {
        state.is_some()
    };
    inspection.enablement = if enabled {
        IndexEnablement::Enabled
    } else {
        IndexEnablement::Disabled
    };
    if let Some((recorded, model, dimension, passages, built_at)) = state {
        let model = EmbeddingModel::try_new(&model).map_err(index_storage_failure)?;
        let dimension = count(dimension)?;
        inspection.built_at = Some(timestamp(built_at)?);
        inspection.passage_count = count(passages)?;
        inspection.freshness = Some(freshness(recorded == hash.as_str()));
        inspection.compatible =
            model == models.embedding_model && dimension == models.embedding_dimension;
        inspection.model = Some(model);
        inspection.dimension = Some(dimension);
    }
    Ok(inspection)
}
