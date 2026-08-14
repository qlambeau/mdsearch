---
id: TABLE-001
title: "Schema Version Table"
type: table-schema
status: approved
created: 2026-08-14
updated: 2026-08-14
owner: TBD
database: DB-001
table_name: "schema_version"
table_type: "table"
related:
  - DB-001
  - DES-001
---

# Table Schema: `schema_version`

## Purpose

The `schema_version` table stores the single integer version denoting the currently
applied database schema migration level for the Collections Database ([`DB-001`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/DB-001.md)).

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `version` | `INTEGER` | No | No | None | Current applied database schema version number (initialized to `1`). |

## Indexes & Constraints

* None explicitly declared. Exactly one row is maintained in this table.

## Invariants & Validation Rules

- Initialized on first connection open:
  ```sql
  INSERT INTO schema_version(version)
  SELECT 1
  WHERE NOT EXISTS (SELECT 1 FROM schema_version);
  ```
- Incremented monotonically when schema migrations are applied in future feature slices.
