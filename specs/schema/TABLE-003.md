---
id: TABLE-003
title: "Files Table"
type: table-schema
status: approved
created: 2026-08-17
updated: 2026-08-17
owner: TBD
database: DB-001
table_name: "files"
table_type: "table"
related:
  - DB-001
  - US-004
  - REQ-004
  - DES-004
---

# Table Schema: `files`

## Purpose

The `files` table stores the content and metadata of ingested markdown files
within the `mdsearch` Collections Database ([`DB-001`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/DB-001.md)).
Each row identifies one file by its canonical absolute path within a collection
and carries its content hash, byte size, and ingest timestamps for later
incremental indexing and retrieval.

## DDL (Schema Definition)

```sql
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
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `file_id` | `INTEGER` | No | Yes | Autoincrement | Stable indexing-assigned file ID retained across re-adds of the same path. |
| `collection_id` | `INTEGER` | No | No | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `path` | `TEXT` | No | No | None | Canonical absolute path of the file; unique within a collection. |
| `content` | `BLOB` | No | No | None | Raw file content bytes. |
| `content_hash` | `TEXT` | No | No | None | Lowercase hexadecimal SHA-256 hash of `content`. |
| `byte_size` | `INTEGER` | No | No | None | Size of `content` in bytes. |
| `created_at` | `INTEGER` | No | No | None | Unix timestamp of first ingestion. |
| `updated_at` | `INTEGER` | No | No | None | Unix timestamp of the most recent ingestion. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_files_1` | `UNIQUE` | `collection_id`, `path` | Enforces one file per canonical path per collection and powers upsert-by-path. |

## Invariants & Validation Rules

- `collection_id` references an existing collection; deleting a collection
  removes its files via `ON DELETE CASCADE`.
- `path` is the canonical absolute path produced by the filesystem adapter.
- `content_hash` equals `SHA-256(content)` as lowercase hexadecimal.
- `byte_size` equals `LENGTH(content)`.
- Re-adding an existing path updates `content`, `content_hash`, `byte_size`,
  and `updated_at`, while preserving `file_id` and `created_at`.
