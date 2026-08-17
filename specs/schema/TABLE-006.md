---
id: TABLE-006
title: "Lexical Index State Table"
type: table-schema
status: approved
created: 2026-08-17
updated: 2026-08-17
owner: TBD
database: DB-001
table_name: "lexical_index_state"
table_type: "table"
related:
  - DB-001
  - US-006
  - REQ-006
  - DES-006
  - TABLE-002
  - TABLE-004
  - TABLE-005
---

# Table Schema: `lexical_index_state`

## Purpose

The `lexical_index_state` table records the aggregate build state of a
collection's lexical index: how many passages were indexed and when the index was
last built. It is the data source for the `mdsearch index status` command and
distinguishes a built index from a collection that has never been updated.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS lexical_index_state (
    collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
    passage_count INTEGER NOT NULL,
    built_at INTEGER NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `collection_id` | `INTEGER` | No | Yes | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `passage_count` | `INTEGER` | No | No | None | Number of passages currently indexed for the collection. |
| `built_at` | `INTEGER` | No | No | None | Unix timestamp of the last successful index build. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_lexical_index_state_1` | `PRIMARY KEY` | `collection_id` | One index state row per collection. |

## Invariants & Validation Rules

- A row exists for a collection only after a successful `collection update`; its
  absence means the index is `NotBuilt`.
- `passage_count` equals the number of `passage_files` rows for the collection at
  the last successful build.
- `built_at` is refreshed on every successful rebuild.
- A collection with zero passages still has a state row after a successful
  update, so it reports `Built` with a passage count of zero.

## Related Tables

- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  is the collection the state belongs to.
- [`TABLE-004`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-004.md)
  and [`TABLE-005`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-005.md)
  hold the passages whose count this table summarizes.