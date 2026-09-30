---
id: TABLE-013
title: "Collection Sources Table"
type: table-schema
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved TABLE-013 in chat on 2026-09-30"
database: DB-001
table_name: "collection_sources"
table_type: "table"
related:
  - DB-001
  - US-021
  - REQ-021
  - DES-021
  - TABLE-002
  - US-016
---

# Table Schema: `collection_sources`

## Purpose

The `collection_sources` table records the canonical filesystem sources
configured for each collection. Updates use these source paths to discover new,
modified, and deleted Markdown files without inferring roots from stored file
paths.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS collection_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER NOT NULL
        REFERENCES collections(collection_id) ON DELETE CASCADE,
    source_path TEXT NOT NULL,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'directory')),
    UNIQUE (collection_id, source_path)
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Foreign Key / Default | Description |
| --- | --- | --- | --- | --- | --- |
| `id` | `INTEGER` | No | Yes | Database-generated | Domain-independent unique row identifier, as required by R-DB-01. |
| `collection_id` | `INTEGER` | No | No | References `collections.collection_id` | Collection that owns this source. |
| `source_path` | `TEXT` | No | No | None | Canonical absolute path to a registered Markdown file or directory. |
| `source_kind` | `TEXT` | No | No | None | `file` or `directory`, captured at registration so updates can distinguish a missing file from an inaccessible directory. |

## Indexes & Constraints

| Name | Type | Target Columns / Expression | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_collection_sources_1` | UNIQUE | `collection_id`, `source_path` | Prevents a collection from registering the same canonical path more than once. |

## Invariants & Validation Rules

- `collection_id` identifies an existing collection.
- `source_path` is absolute and canonicalized by the filesystem boundary before persistence.
- `source_kind` is captured as `file` or `directory` when registered and is never guessed from stored file rows.
- A collection may have zero source rows, including legacy collections migrated from earlier schemas.
- Removing a collection removes its source rows as part of collection-owned data cleanup.
- The table is introduced by database schema version 8 and migration inserts no source rows for existing collections.

## Related Tables

- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  identifies the collection that owns each source.
- [`TABLE-003`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-003.md)
  stores the file content discovered beneath registered sources.
