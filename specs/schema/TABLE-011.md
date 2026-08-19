---
id: TABLE-011
title: "Graph Edges Table"
type: table-schema
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
database: DB-001
table_name: "edges"
table_type: "table"
related:
  - DB-001
  - US-012
  - REQ-012
  - DES-012
  - ADR-008
  - TABLE-002
  - TABLE-010
  - TABLE-012
---

# Table Schema: `edges`

## Purpose

The `edges` table stores the typed, directional relationships between graph
nodes built deterministically by `mdsearch update` (EPIC-005). Five relation
types are supported: `LINKS_TO` (inline relative `.md` link), `TAGGED_WITH`
(file to tag), `ALIAS_OF` (file to alias), `RELATED_TO` (frontmatter `related:`),
and `HAS_SOURCE` (frontmatter `sources:`). Each edge references source and
destination nodes from [`TABLE-010`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-010.md).

## DDL (Schema Definition)

```sql
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
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `edge_id` | `INTEGER` | No | Yes | Autoincrement | Surrogate identity of the edge. |
| `collection_id` | `INTEGER` | No | No | None | Foreign key to `collections.collection_id`; cascades on collection destroy. |
| `src_id` | `INTEGER` | No | No | None | Source node, references `nodes.node_id`; cascades when its node is deleted. |
| `dst_id` | `INTEGER` | No | No | None | Destination node, references `nodes.node_id`; cascades when its node is deleted. |
| `relation` | `TEXT` | No | No | None | Closed relation type set: `LINKS_TO`, `TAGGED_WITH`, `ALIAS_OF`, `RELATED_TO`, `HAS_SOURCE`. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_edges_1` | `UNIQUE` | `collection_id`, `src_id`, `dst_id`, `relation` | One edge per directed relation per collection; powers idempotent rebuilds. |
| `idx_edges_src` | `INDEX` | `src_id`, `relation` | Fast neighbor expansion from a source node. |
| `idx_edges_dst` | `INDEX` | `dst_id`, `relation` | Fast reverse lookups to a destination node. |

## Invariants & Validation Rules

- Every edge connects a `file` source node to a destination node: `TAGGED_WITH`
  and `ALIAS_OF` connect a file to a `tag`/`alias`; `LINKS_TO`, `RELATED_TO`, and
  `HAS_SOURCE` connect a file to another `file`.
- The relation type is drawn from the closed set; no other relation is stored.
- An unresolved `related:`/`sources:` reference or inline link target (one that
  does not match a known file `node_key`) produces no edge (REQ-012 FR-010).
- The `UNIQUE (collection_id, src_id, dst_id, relation)` constraint guarantees a
  full deterministic rebuild produces no duplicate edges.
- Deleting a node cascades to its incident edges, so stale edges from deleted
  files disappear automatically (REQ-012 FR-011).

## Related Tables

- [`TABLE-010`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-010.md)
  holds the source and destination node rows referenced by `src_id` and `dst_id`.
- [`TABLE-002`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-002.md)
  is the collection the edge belongs to.
- [`TABLE-012`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-012.md)
  records the per-collection build state the rebuild reads and writes.
