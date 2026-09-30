---
id: TABLE-002
title: "Collections Table"
type: table-schema
status: approved
created: 2026-08-14
updated: 2026-09-30
owner: Quentin
database: DB-001
table_name: "collections"
table_type: "table"
related:
  - DB-001
  - US-001
  - REQ-001
  - DES-001
  - US-022
  - REQ-022
  - DES-022
  - ADR-016
---

# Table Schema: `collections`

## Purpose

The `collections` table stores the persistent identity and metadata for named collections
within the `mdsearch` Collections Database ([`DB-001`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/DB-001.md)). It enforces case-insensitive name uniqueness
while preserving the original user-supplied display casing.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS collections (
    collection_id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL,
    name_key TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    semantic_enabled INTEGER NOT NULL DEFAULT 0 CHECK (semantic_enabled IN (0, 1))
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `collection_id` | `INTEGER` | No | Yes | Autoincrement | Stable internal primary key for future relationships (files, embeddings, graph nodes). |
| `display_name` | `TEXT` | No | No | None | Trimmed original casing of the collection name retained for human-readable output. |
| `name_key` | `TEXT` | No | No | None | Unicode-aware case-folded canonical string used for case-insensitive uniqueness comparison. |
| `created_at` | `INTEGER` | No | No | None | Unix timestamp (seconds since epoch) when the collection was created. |
| `semantic_enabled` | `INTEGER` | No | No | `0` | Whether source-aware updates build a semantic index for this collection. Only `0` and `1` are valid; new collections default to disabled. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_collections_1` | `UNIQUE` | `name_key` | Enforces case-insensitive collection name uniqueness within the database file. |

## Invariants & Validation Rules

- `display_name` is validated prior to insertion:
  - Must not be empty or whitespace-only after trimming.
  - Must not contain path separators (`/`, `\`).
  - Must not contain ASCII or Unicode control characters.
- `name_key` is deterministically derived from `display_name` via Unicode case folding.
- `semantic_enabled` is the persisted semantic-index policy; disabling it removes vectors and semantic-index state on the next successful update.
- Migration from schema version 8 sets `semantic_enabled` to `1` only for collections that already have a `semantic_index_state` row; all other collections remain at `0`.
- Insertions and uniqueness checks are executed within a single transaction to guarantee atomic creation without partial state.
