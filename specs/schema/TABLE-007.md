---
id: TABLE-007
title: "Settings Table"
type: table-schema
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
database: DB-001
table_name: "settings"
table_type: "table"
related:
  - DB-001
  - US-010
  - REQ-010
  - DES-010
  - ADR-006
---

# Table Schema: `settings`

## Purpose

The `settings` table stores database-wide key/value configuration. It holds the
single global embedding model (`embed_model`) used by `mdsearch embed`, ensuring
every collection's vectors are comparable under one model, and the active
vector-table dimension (`embedding_dimension`) of the shared `embeddings`
virtual table.

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `key` | `TEXT` | No | Yes | None | The setting name, e.g. `embed_model`. |
| `value` | `TEXT` | No | No | None | The setting value, e.g. the global embedding model name. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| `sqlite_autoindex_settings_1` | `PRIMARY KEY` | `key` | One value per setting name. |

## Invariants & Validation Rules

- The `embed_model` key, when present, holds the name of the single global
  embedding model.
- The `embedding_dimension` key, when present, holds the dimension of the
  shared `embeddings` vector table; its absence means the legacy default of
  384.
- The absence of the `embed_model` key means no global model has been recorded
  yet (first `embed` run has not happened or no model was ever set).
- The values are written only by the embed store; other commands never modify
  them.

## Related Tables

- [`TABLE-008`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-008.md)
  records each collection's embed state under the global model.
- [`TABLE-009`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-009.md)
  stores the vectors generated under the global model.
