---
id: TABLE-009
title: "Embeddings Vector Table"
type: table-schema
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
database: DB-001
table_name: "embeddings"
table_type: "virtual table (sqlite-vector)"
related:
  - DB-001
  - US-010
  - REQ-010
  - DES-010
  - ADR-006
  - TABLE-004
  - TABLE-005
  - TABLE-008
---

# Table Schema: `embeddings`

## Purpose

The `embeddings` virtual table stores one vector per indexed passage, keyed to
the stable logical passage identity `(file_id, kind, position)` rather than to
the physical FTS5 passage rowid. Because `update` rebuilds the lexical index on
every run (ADR-005), passage rowids change even when file content is unchanged;
the logical key keeps semantic vectors valid across such rebuilds and lets
hybrid search join semantic results back to the lexical `passage_files` rows.

## DDL (Schema Definition)

```sql
CREATE VIRTUAL TABLE embeddings USING vector(
    dim=384,
    type=float4,
    metric=cosine,
    metadata='collection_id INTEGER, file_id INTEGER, kind TEXT, position INTEGER'
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `rowid` | `INTEGER` | No | Yes | None | The vector table row identifier assigned by the extension. |
| `vector` | `BLOB` | No | No | None | The embedded vector (384 float4 elements), cosine-normalized. |
| `collection_id` | `INTEGER` | No | No | None | Metadata column; owning collection, enabling per-collection rebuild and later filtering. |
| `file_id` | `INTEGER` | No | No | None | Metadata column; the owning file in `files`. |
| `kind` | `TEXT` | No | No | None | Metadata column; passage kind (`body`, `title`, `tags`, `aliases`, or `summary`). |
| `position` | `INTEGER` | No | No | None | Metadata column; zero-based ordinal of the passage within its file. |

## Indexes & Constraints

- The `sqlite-vector` extension maintains the HNSW graph and shadow data tables;
  no application indexes are defined on the virtual table.
- `(file_id, kind, position)` is unique within a collection and forms the stable
  logical key matching the `passage_files` rows.

## Invariants & Validation Rules

- One row exists per indexed passage for every collection whose `embed` state
  exists in `semantic_index_state`.
- `collection_id`, `file_id`, `kind`, and `position` match the corresponding
  `passage_files` row for the same passage, so semantic results can be joined
  back to lexical passages and files.
- The vector dimension matches the embedding model's output (384 for
  `all-MiniLM-L6-v2`); the adapter asserts the dimension before insert.
- Rebuilding a collection deletes all of its rows (`DELETE ... WHERE
  collection_id = ?`) and reinserts the current passage vectors in one
  transaction with the `semantic_index_state` update.

## Related Tables

- [`TABLE-004`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-004.md)
  and [`TABLE-005`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-005.md)
  define the lexical passages these vectors correspond to.
- [`TABLE-008`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-008.md)
  records the aggregate embed state (fingerprint, model, counts).
- [`TABLE-007`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-007.md)
  holds the global model under which these vectors were generated.
