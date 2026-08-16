---
id: TABLE-NNN
title: "Table title"
type: table-schema
status: draft
created: YYYY-MM-DD
updated: YYYY-MM-DD
owner: TBD
database: DB-NNN
table_name: "tbd_table"
table_type: "table" # table | virtual_table | view
related: []
---

# Table Schema

<!-- Supporting artifact. Defines a single database table or virtual table,
its column specifications, constraints, indexes, and domain invariants. -->

## Purpose

TBD

## DDL (Schema Definition)

```sql
CREATE TABLE IF NOT EXISTS tbd_table (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Foreign Key / Default | Description |
| --- | --- | --- | --- | --- | --- |
| `id` | `INTEGER` | No | Yes | None | Unique identifier |
| `name` | `TEXT` | No | No | None | Name field |

## Indexes & Constraints

| Name | Type | Target Columns / Expression | Purpose |
| --- | --- | --- | --- |
| `tbd_idx` | UNIQUE | `name` | Enforces uniqueness |

## Invariants & Validation Rules

- TBD
