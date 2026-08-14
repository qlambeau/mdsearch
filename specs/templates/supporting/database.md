---
id: DB-NNN
title: "Database title"
type: database-schema
status: draft
created: YYYY-MM-DD
updated: YYYY-MM-DD
owner: TBD
engine: "SQLite 3"
file_path: "TBD"
tables:
  - TABLE-NNN
related: []
---

# Database Schema

<!-- Supporting artifact. Defines a physical/logical database instance,
its engine, configuration, migration strategy, and associated tables. -->

## Database Overview

- **Engine:** SQLite 3 (or TBD)
- **Default Location:** `TBD`
- **Override Flag:** `TBD`
- **Concurrency & Locking:** `TBD`

## Configuration & Extensions

- **Extensions:** TBD (e.g. `sqlite-vector`, FTS5)
- **PRAGMAs / Settings:** TBD (e.g. `journal_mode=WAL`, `foreign_keys=ON`)

## Schema Evolution & Migrations

| Version | Applied In | Description |
| --- | --- | --- |
| 1 | TBD | Initial schema creation |

## Table Catalog

| Table Name | Spec ID | Type | Description |
| --- | --- | --- | --- |
| `tbd_table` | `TABLE-NNN` | Table / Virtual Table | TBD |
