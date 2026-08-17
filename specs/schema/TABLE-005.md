---
id: TABLE-005
title: "Passage Files Mapping Table"
type: table-schema
status: approved
created: 2026-08-17
updated: 2026-08-17
owner: TBD
database: DB-001
table_name: "passage_files"
table_type: "table"
related:
  - DB-001
  - US-006
  - REQ-006
  - DES-006
  - TABLE-003
  - TABLE-004
---

# Table Schema: `passage_files`

## Purpose

The `passage_files` mapping table links each FTS5 row in
[`passages`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-004.md)
back to the owning collection, file, passage kind, and ordinal position. FTS5
rows cannot be filtered by collection or file directly, so this table powers
per-collection rebuilds (delete stale rows), the `index status` passage counts,
and future retrieval slices that need to locate a matched passage within its
file.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS passage_files (
    passage_rowid INTEGER PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    position INTEGER NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `passage_rowid` | `INTEGER` | No | Yes | None | The `rowid` of the row in the `passages` FTS5 table. |
| `collection_id` | `INTEGER` | No | No | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `file_id` | `INTEGER` | No | No | None | The `file_id` of the owning file in `files`; removed when the file is deleted. |
| `kind` | `TEXT` | No | No | None | Passage kind: `body`, `title`, `tags`, `aliases`, or `summary`. |
| `position` | `INTEGER` | No | No | None | Zero-based ordinal of the passage within its file for stable ordering. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_passage_files_1` | `PRIMARY KEY` | `passage_rowid` | One mapping per FTS5 row. |
| index on `collection_id` | Index | `collection_id` | Fast per-collection row lookup and deletion during rebuild. |

## Invariants & Validation Rules

- `passage_rowid` references an existing row in `passages`; the adapter records
  `last_insert_rowid()` after each FTS5 insert.
- `collection_id` references an existing collection; deleting a collection
  removes its mappings via `ON DELETE CASCADE`.
- `file_id` references `files.file_id`; a file deletion removes its passages in
  the same rebuild that reconciles the file set.
- `kind` is one of the four recognized frontmatter fields or `body`.
- `position` is unique within `(file_id, kind)` ordering context and assigns a
  stable ordinal to every passage of a file.

## Related Tables

- [`TABLE-003`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-003.md)
  is the file the passages belong to.
- [`TABLE-004`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-004.md)
  holds the tokenized passage text keyed by `passage_rowid`.
- [`TABLE-006`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-006.md)
  records the aggregate index state for each collection.