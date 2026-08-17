---
id: TABLE-004
title: "Passages FTS5 Virtual Table"
type: table-schema
status: approved
created: 2026-08-17
updated: 2026-08-17
owner: TBD
database: DB-001
table_name: "passages"
table_type: "virtual table (FTS5)"
related:
  - DB-001
  - US-006
  - REQ-006
  - DES-006
  - ADR-005
  - TABLE-005
---

# Table Schema: `passages`

## Purpose

The `passages` virtual table stores one full-text row per indexed passage of the
`mdsearch` Collections Database ([`DB-001`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/DB-001.md)).
Each row holds the tokenizable text of one body paragraph or one recognized
frontmatter field. FTS5 builds and maintains the inverted index over `content`
and later slices score rows with the `bm25()` ranking function. The table is
populated only by `collection update` and is rebuilt in full for a collection on
each update.

## DDL (Schema Definition)

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS passages USING fts5(
    content,
    tokenize = 'unicode61'
);
```

## Column Specifications

| Column | Data Type | Nullable | Primary Key | Default | Description |
| --- | --- | --- | --- | --- | --- |
| `content` | `TEXT` | No | No | None | Tokenizable text of one passage (one body paragraph or one frontmatter field value). |
| `rowid` | `INTEGER` | No | Yes (implicit) | Autoincrement | Implicit FTS5 rowid; referenced by `passage_files.passage_rowid`. |

## Indexes & Constraints

| Name | Type | Target Columns | Purpose |
| --- | --- | --- | --- |
| implicit FTS5 inverted index | FTS5 | `content` | Full-text tokenization and matching over `content` with the `unicode61` tokenizer. |

## Invariants & Validation Rules

- Rows exist only for collections whose index has been built by a successful
  update.
- Every row is referenced by exactly one `passage_files` row so the owning
  collection, file, kind, and position can be resolved.
- The FTS5 rowid is unique and recorded in `passage_files.passage_rowid`.
- The full set of rows for a collection is replaced on every rebuild of that
  collection's index.

## Related Tables

- [`TABLE-005`](file:///home/quentin/Documents/dev/genAI/code/kv/specs/schema/TABLE-005.md)
  maps each `passages` rowid back to its collection, file, kind, and position.