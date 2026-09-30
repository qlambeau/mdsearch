//! `SQLite` schema persistence.

use rusqlite::Connection;
use sqlite_vector_rs::scalar;
use sqlite_vector_rs::vtab::{Registry, VectorTable};
use sqlite3_ext::Connection as ExtensionConnection;
use sqlite3_ext::vtab::{Module, StandardModule};

/// The current database schema version applied by [`migrate`].
pub(super) const CURRENT_SCHEMA_VERSION: i64 = 9;

/// The schema version at which the lexical index tables exist.
pub(super) const INDEX_SCHEMA_VERSION: i64 = 3;

/// The dimension assumed for databases that predate dimension recording.
pub(super) const LEGACY_EMBEDDING_DIMENSION: i64 = 384;

pub(crate) fn register_vector_extension(connection: &Connection) -> Result<(), sqlite3_ext::Error> {
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
pub(super) fn migrate(connection: &Connection) -> Result<(), rusqlite::Error> {
    create_initial_tables(connection)?;

    create_graph_tables(connection)?;

    let version: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if version < CURRENT_SCHEMA_VERSION {
        if version < 9 && !table_has_column(connection, "collections", "semantic_enabled")? {
            connection.execute(
                "ALTER TABLE collections ADD COLUMN semantic_enabled INTEGER NOT NULL DEFAULT 0 CHECK (semantic_enabled IN (0, 1))",
                [],
            )?;
            connection.execute(
                "UPDATE collections SET semantic_enabled = 1
                 WHERE EXISTS (
                     SELECT 1 FROM semantic_index_state state
                     WHERE state.collection_id = collections.collection_id
                 )",
                [],
            )?;
        }
        if version < 4 && !table_has_column(connection, "passage_files", "byte_offset")? {
            connection.execute(
                "ALTER TABLE passage_files ADD COLUMN byte_offset INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        if version < 7 && !table_has_column(connection, "semantic_index_state", "dimension")? {
            connection.execute(
                "ALTER TABLE semantic_index_state ADD COLUMN dimension INTEGER",
                [],
            )?;
        }
        connection.execute("DELETE FROM schema_version", [])?;
        connection.execute(
            "INSERT INTO schema_version(version) VALUES (?1)",
            [CURRENT_SCHEMA_VERSION],
        )?;
    }

    Ok(())
}

/// Creates the entity-graph schema tables if absent.
pub(super) fn create_graph_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS nodes (
            node_id INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
            node_kind TEXT NOT NULL CHECK (node_kind IN ('file', 'tag', 'alias')),
            node_key TEXT NOT NULL,
            title TEXT NOT NULL,
            UNIQUE (collection_id, node_kind, node_key)
        );
        CREATE TABLE IF NOT EXISTS edges (
            edge_id INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
            src_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
            dst_id INTEGER NOT NULL REFERENCES nodes(node_id) ON DELETE CASCADE,
            relation TEXT NOT NULL CHECK (relation IN ('LINKS_TO', 'TAGGED_WITH', 'ALIAS_OF', 'RELATED_TO', 'HAS_SOURCE')),
            UNIQUE (collection_id, src_id, dst_id, relation)
        );
        CREATE INDEX IF NOT EXISTS idx_edges_src ON edges(src_id, relation);
        CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst_id, relation);
        CREATE TABLE IF NOT EXISTS graph_state (
            collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
            file_set_fingerprint TEXT NOT NULL,
            node_count INTEGER NOT NULL,
            edge_count INTEGER NOT NULL,
            built_at INTEGER NOT NULL
        );",
    )
}

/// Returns whether `table` has a column named `column`.
pub(super) fn table_has_column(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, rusqlite::Error> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for name in columns {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn schema_version(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
}

fn create_initial_tables(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS collections (
            collection_id INTEGER PRIMARY KEY,
            display_name TEXT NOT NULL,
            name_key TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL,
            semantic_enabled INTEGER NOT NULL DEFAULT 0 CHECK (semantic_enabled IN (0, 1))
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
            position INTEGER NOT NULL,
            byte_offset INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_passage_files_collection
            ON passage_files(collection_id);
        CREATE TABLE IF NOT EXISTS lexical_index_state (
            collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
            passage_count INTEGER NOT NULL,
            built_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS semantic_index_state (
            collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
            file_set_fingerprint TEXT NOT NULL,
            model TEXT NOT NULL,
            passage_count INTEGER NOT NULL,
            embedded_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS collection_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
            source_path TEXT NOT NULL,
            source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'directory')),
            UNIQUE(collection_id, source_path)
        );",
    )?;

    Ok(())
}
