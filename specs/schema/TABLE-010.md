---
id: TABLE-010
title: "Graph Nodes Table"
type: table-schema
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
database: DB-001
table_name: "nodes"
table_type: "table"
related:
  - DB-001
  - US-012
  - REQ-012
  - DES-012
  - ADR-008
  - TABLE-002
  - TABLE-003
  - TABLE-011
  - TABLE-012
---

# Table Schema: `nodes`

## Purpose

The `nodes` table stores the entity nodes of a collection's contextual graph,
built deterministically by `mdsearch update` (EPIC-005). A node is one of three
kinds: a `file` (a stored markdown file), a `tag` (a frontmatter `tags:` value),
or an `alias` (a frontmatter `aliases:` value). The table gives every node a
stable identity so the `edges` table can reference it.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS nodes (
    node_id INTEGER PRIMARY KEY,
    collection_id INTEGER NOT NULL REFERENCES collections(collection_id) ON DELETE CASCADE,
    node_kind TEXT NOT NULL CHECK (node_kind IN ('file', 'tag', 'alias')),
    node_key TEXT NOT NULL,
    title TEXT NOT NULL,
    UNIQUE (collection_id, node_kind, node_key)
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `node_id` | `INTEGER` | No | Yes | Autoincrement | Surrogate identity of the node within the database. |
| `collection_id` | `INTEGER` | No | No | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `node_kind` | `TEXT` | No | No | None | Discriminator: `file`, `tag`, or `alias`. |
| `node_key` | `TEXT` | No | No | None | Stable identity of the node. For `file` nodes the canonical file path; for `tag`/`alias` nodes the exact normalized name. |
| `title` | `TEXT` | No | No | None | Human label of the node: the file name, the tag name, or the alias name. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_nodes_1` | `UNIQUE` | `collection_id`, `node_kind`, `node_key` | One node per identity per collection; powers idempotent rebuilds and edge joins. |

## Invariants & Validation Rules

- A `file` node exists for every stored file in the collection, identified by its
  canonical path (`node_key`); its `title` is the file's display name.
- A `tag` node exists for every distinct frontmatter `tags:` value, identified
  by the exact normalized tag name; its `title` is the tag name.
- An `alias` node exists for every distinct frontmatter `aliases:` value,
  identified by the exact normalized alias name; its `title` is the alias name.
- A tag node and an alias node with the same `node_key` remain distinct because
  `node_kind` differs (REQ-012 FR-013).
- The `(collection_id, node_kind, node_key)` uniqueness guarantees a full
  deterministic rebuild produces no duplicate nodes.

## Related Tables

- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  is the collection the node belongs to.
- [`TABLE-003`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-003.md)
  provides the stored files that become `file` nodes.
- [`TABLE-011`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-011.md)
  holds the edges that reference these nodes.
- [`TABLE-012`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-012.md)
  records the per-collection build state the rebuild reads and writes.
