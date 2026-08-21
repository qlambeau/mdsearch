---
id: US-016
title: "Destroy a collection completely with no orphaned data"
type: user-story
status: approved
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: PRD-001
epic: EPIC-010
feature: 016-destroy-integrity
related:
  - US-003
  - US-006
  - US-010
  - US-012
---

# User Story

## Story Card

As a developer-curator,
I want `mdsearch collection destroy` to remove every trace of a collection —
files, passages, vectors, graph nodes/edges, and index state —
so that no orphaned data remains in the database and a newly created
collection can never inherit a destroyed one's data.

## Context And Value

`destroy_collection` currently executes a single statement:
`DELETE FROM collections WHERE name_key = ?1` (`store-sqlite/lib.rs:314`).
SQLite does not enforce the `ON DELETE CASCADE` clauses declared on the child
tables because `PRAGMA foreign_keys = ON` is never executed on connection
opening, and virtual tables (`passages` FTS5, `embeddings` sqlite-vector)
cannot declare foreign keys at all. Destroying a collection therefore leaves
its stored files, full-text rows, mapping rows, vectors, graph nodes and
edges, and index-state records orphaned permanently in the database (OBS-008).

Because SQLite reuses rowids after deletion, a newly created collection can
receive the same `collection_id` as a destroyed one and inherit the orphaned
files, passages, embeddings, and graph rows — corrupting later searches,
hybrid results, and graph queries. Complete cleanup at destroy time removes
both the storage waste and the inheritance hazard, and the explicit
transactional delete works regardless of foreign-key enforcement.

## Business Rules

- Destroying a collection removes its rows from every table in one atomic
  transaction: `collections`, `files`, `passages` (FTS5), `passage_files`,
  `embeddings`, `nodes`, `edges`, `graph_state`, `lexical_index_state`, and
  `semantic_index_state`.
- The destroy is all-or-nothing: a failure rolls back and leaves the
  collection and all of its data intact.
- Destroying an unknown collection reports the existing collection-not-found
  error and changes nothing.
- Destroying one collection never touches the files, indexes, or graph of
  other collections.
- Cleanup is by explicit per-table deletes; enabling `PRAGMA foreign_keys =
  ON` on connections remains a separate observation (OBS-011) and is not a
  prerequisite of this behavior.
- The database file itself is not deleted, and freed space is not reclaimed:
  the rows are gone, but the file may not shrink (VACUUM is out of scope).

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A fully indexed collection (files, lexical, semantic, graph) exists | I run `mdsearch collection destroy Notes` | Every child, virtual, and state table holds zero rows for that collection |
| EX-002 | A collection with only ingested files exists | I destroy it | Its files are gone; no orphan rows remain anywhere |
| EX-003 | No collection named `Journal` exists | I destroy `Journal` | The command fails with the collection-not-found error; nothing changes |
| EX-004 | Collections `Notes` and `Archive` both have indexed data | I destroy `Notes` | `Archive`'s files, indexes, and graph are fully intact |
| EX-005 | `Notes` is destroyed, then a new collection is created (possibly reusing the rowid) | I search, hybrid, or graph-query the new collection | No stale passages, vectors, or graph rows surface from the destroyed collection |

## Acceptance Criteria

- After a successful destroy, the database contains zero rows for the
  destroyed collection in all ten tables (`collections`, `files`, `passages`,
  `passage_files`, `embeddings`, `nodes`, `edges`, `graph_state`,
  `lexical_index_state`, `semantic_index_state`), verified by direct SQL in
  store integration tests.
- The destroy is atomic: an injected failure mid-delete leaves the collection
  and its data intact.
- Destroying an unknown collection fails with the existing not-found error and
  changes nothing.
- Destroying one collection leaves other collections' data fully intact.
- A new collection created after a destroy never surfaces stale passages,
  vectors, or graph rows from the destroyed collection.
- Regression scenarios are added to the `003-destroy-collection` feature
  packet.

## Scope Boundaries

### In Scope

- Explicit multi-table transactional delete in `destroy_collection`.
- Regression integration tests verifying table emptiness after destroy and
  atomicity on failure.
- Regression scenarios in the `003-destroy-collection` packet.

### Out Of Scope

- Enabling `PRAGMA foreign_keys = ON` on connection open (OBS-011).
- VACUUM or physical space reclamation of the database file.
- Deleting the database file itself.
- Changing the destroy command's CLI surface or error contract.
- Other TODO.md observations (OBS-004, OBS-005, ...).

## Dependencies

- `US-003` (EPIC-001) provides the `destroy` command whose behavior this story
  completes.
- `US-006`, `US-010`, and `US-012` (EPIC-002/004/005) own the tables whose
  orphaned rows this story cleans up.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Should the database file's freed space be reclaimed (VACUUM) after destroy? | No | TBD | Out of scope; noted for a possible later slice |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 2 days
- [x] Testable