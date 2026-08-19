---
id: TABLE-012
title: "Graph State Table"
type: table-schema
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
database: DB-001
table_name: "graph_state"
table_type: "table"
related:
  - DB-001
  - US-012
  - REQ-012
  - DES-012
  - ADR-008
  - TABLE-002
  - TABLE-010
  - TABLE-011
---

# Table Schema: `graph_state`

## Purpose

The `graph_state` table records the aggregate build state of a collection's
contextual entity graph: the stored file-set fingerprint the graph was built
from, the node and edge counts, and the last build timestamp. It mirrors the
state-table pattern of `lexical_index_state` ([`TABLE-006`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-006.md))
and `semantic_index_state` ([`TABLE-008`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-008.md)),
and lets `mdsearch update` decide whether a collection's graph is already current
and later slices detect staleness.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS graph_state (
    collection_id INTEGER PRIMARY KEY REFERENCES collections(collection_id) ON DELETE CASCADE,
    file_set_fingerprint TEXT NOT NULL,
    node_count INTEGER NOT NULL,
    edge_count INTEGER NOT NULL,
    built_at INTEGER NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `collection_id` | `INTEGER` | No | Yes | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `file_set_fingerprint` | `TEXT` | No | No | None | Hash of the stored file set (paths and content hashes) the graph was built from. |
| `node_count` | `INTEGER` | No | No | None | Number of `nodes` rows for the collection at last build. |
| `edge_count` | `INTEGER` | No | No | None | Number of `edges` rows for the collection at last build. |
| `built_at` | `INTEGER` | No | No | None | Unix timestamp of the last successful graph build. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_graph_state_1` | `PRIMARY KEY` | `collection_id` | One graph state row per collection. |

## Invariants & Validation Rules

- A row exists for a collection only after a successful graph build via
  `mdsearch update`; its absence means the graph has never been built.
- `node_count` equals the number of `nodes` rows for the collection at last
  build, and `edge_count` equals the number of `edges` rows.
- `file_set_fingerprint` equals the fingerprint of the stored `files` table
  contents at last build; a change means the collection must be rebuilt.
- On every `update`, the build replaces the collection's nodes, edges, and this
  state row in one transaction, so stale state never persists.

## Related Tables

- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  is the collection the state belongs to.
- [`TABLE-010`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-010.md)
  and [`TABLE-011`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-011.md)
  hold the nodes and edges whose counts this table summarizes.
