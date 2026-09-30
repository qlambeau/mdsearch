//! `SQLite` lexical build persistence.

use kv_application::FileStoreError;
use kv_domain::{FrontmatterIssue, segment_passages};
use rusqlite::{Transaction, params};

use crate::file_storage_failure;

/// Rebuilds the lexical index for a collection from its stored files.
///
/// Deletes the collection's existing passages and reinserts them from the
/// current `files` rows, then records the index state. Returns the number of
/// files whose frontmatter could not be parsed.
pub(super) fn rebuild_index(
    transaction: &Transaction<'_>,
    collection_id: i64,
    built_at: i64,
) -> Result<usize, FileStoreError> {
    clear_lexical_passages(transaction, collection_id)?;

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
            let byte_offset = i64::try_from(passage.byte_offset()).map_err(file_storage_failure)?;
            transaction
                .execute(
                    "INSERT INTO passage_files(passage_rowid, collection_id, file_id, kind, position, byte_offset)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        rowid,
                        collection_id,
                        file_id,
                        passage.kind().as_str(),
                        position,
                        byte_offset,
                    ],
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

fn clear_lexical_passages(
    transaction: &Transaction<'_>,
    collection_id: i64,
) -> Result<(), FileStoreError> {
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

    Ok(())
}
