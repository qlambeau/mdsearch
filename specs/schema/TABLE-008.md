---
id: TABLE-008
title: "Semantic Index State Table"
type: table-schema
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
database: DB-001
table_name: "semantic_index_state"
table_type: "table"
related:
  - DB-001
  - US-010
  - REQ-010
  - DES-010
  - ADR-006
  - TABLE-002
  - TABLE-009
---

# Table Schema: `semantic_index_state`

## Purpose

The `semantic_index_state` table records the aggregate build state of a
collection's semantic (vector) index: the stored file-set fingerprint the
vectors were built from, the embedding model used, the number of embedded
passages, and when the index was last embedded. It lets `mdsearch embed` decide
whether a collection is already current and later slices detect stale semantic
indexes.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS semantic_index_state (
    collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
    file_set_fingerprint TEXT NOT NULL,
    model TEXT NOT NULL,
    passage_count INTEGER NOT NULL,
    embedded_at INTEGER NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `collection_id` | `INTEGER` | No | Yes | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `file_set_fingerprint` | `TEXT` | No | No | None | Hash of the stored file set (paths and content hashes) the vectors were built from. |
| `model` | `TEXT` | No | No | None | Embedding model the vectors were generated with. |
| `passage_count` | `INTEGER` | No | No | None | Number of passages embedded for the collection. |
| `embedded_at` | `INTEGER` | No | No | None | Unix timestamp of the last successful embed. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_semantic_index_state_1` | `PRIMARY KEY` | `collection_id` | One semantic state row per collection. |

## Invariants & Validation Rules

- A row exists for a collection only after a successful `mdsearch embed`; its
  absence means the semantic index has never been built for the collection.
- `passage_count` equals the number of `embeddings` rows for the collection at
  the last successful embed.
- `file_set_fingerprint` equals the fingerprint of the stored `files` table
  contents at the last successful embed; a change means the collection must be
  rebuilt.
- `model` matches the global `embed_model` setting at the last successful embed;
  a `--model` switch changes both the global setting and this column for every
  rebuilt collection.

## Related Tables

- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  is the collection the state belongs to.
- [`TABLE-009`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-009.md)
  holds the vectors whose count this table summarizes.
- [`TABLE-007`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-007.md)
  holds the global model this state must match.
