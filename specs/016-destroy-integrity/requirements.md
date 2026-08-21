---
id: REQ-016
title: "Destroy a collection completely with no orphaned data requirements"
type: feature-requirements
status: draft
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-016
related:
  - US-003
  - US-006
  - US-010
  - US-012
  - REQ-003
  - REQ-006
  - REQ-010
  - REQ-012
---

# Requirements

## Purpose And Actors

### Purpose

Make `mdsearch collection destroy` remove every trace of the destroyed
collection — stored files, FTS5 passages, passage mappings, vectors, graph
nodes and edges, and all index-state rows — in one atomic transaction, so no
orphaned data remains in the database and a newly created collection can never
inherit a destroyed one's data. The feature completes EPIC-010.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.

## Preconditions

- The user invokes `mdsearch collection destroy NAME` with the existing CLI
  contract from `REQ-003` (case-insensitive name matching, name validation,
  `--database PATH`).
- The database exists; the name validation and missing-database behavior from
  `REQ-003` remain in force.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Destroy a collection | Required `NAME`; optional `--database PATH` | Human-readable confirmation naming the destroyed collection; the collection and every row belonging to it are removed | Name valid and matched case-insensitively against an existing collection |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | A successful destroy shall remove the matching collection's rows from every table that can hold per-collection data — `collections`, `files`, `passages`, `passage_files`, `embeddings`, `nodes`, `edges`, `graph_state`, `lexical_index_state`, and `semantic_index_state` — in one transaction. | Must | US-016; Destroying a fully indexed collection removes every trace; Destroying a collection with only files removes its files |
| FR-002 | The destroy shall be atomic: a failure at any point rolls back the whole delete and leaves the collection and all of its data intact. | Must | US-016; A failed destroy leaves the collection intact |
| FR-003 | Destroying an unknown collection shall fail with the existing not-found error and change nothing. | Must | US-016; Destroying an unknown collection changes nothing; REQ-003 FR-006 |
| FR-004 | Destroying one collection shall leave the files, indexes, graph, and state of every other collection untouched. | Must | US-016; Destroying one collection leaves others fully intact; REQ-003 FR-009 |
| FR-005 | A collection created after a destroy (including one that receives the destroyed collection's reused rowid) shall never surface stale passages, vectors, or graph rows from the destroyed collection. | Must | US-016; Recreating a collection after destroy surfaces no stale data |
| FR-006 | The cleanup shall be performed by explicit per-table deletes; it shall not rely on SQLite foreign-key cascades (`PRAGMA foreign_keys = ON` remains deferred to OBS-011). | Must | US-016 (business rules); DEC-015 |
| FR-007 | The destroy shall not delete the database file and shall not reclaim freed space (no VACUUM). | Must | US-016 (scope boundaries) |
| FR-008 | The destroy command's CLI surface and error contract from `REQ-003` (name validation, case-insensitive matching, missing-database behavior, no confirmation prompt, confirmation output) shall remain unchanged. | Must | US-016 (scope boundaries); REQ-003 |

## Postconditions And Invariants

- After a successful destroy, the database contains zero rows belonging to the
  destroyed collection in all ten per-collection tables.
- The destroy is all-or-nothing: the collection and its data are either fully
  present or fully absent.
- Exactly one collection is removed per successful destroy; no other table
  content changes.
- No foreign-key cascade or trigger is required for the cleanup to be complete.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Unknown collection name | Fail before any delete | Existing collection-not-found error; database unchanged |
| Storage failure mid-destroy | Roll back the transaction | The collection and its data remain intact; clear storage error |
| Invalid name (empty, whitespace, path separator, control character) | Reject before any delete (unchanged) | Existing invalid-name error |
| The database does not exist | Fail without creating a file (unchanged) | Existing missing-database error |
| The collection rowid is later reused by a new collection | New collection starts clean | No stale passages, vectors, or graph rows |

## Quality Requirements

- Completeness is deterministic and structural: the delete list enumerates
  every per-collection table, so no schema addition can silently orphan data
  without a corresponding delete clause (covered by store regression tests).
- Atomicity is guaranteed by a single SQLite transaction; no partial destroy
  state is observable.
- The destroy operates fully offline and requires no network or external
  service.

## Dependencies And Deferred Decisions

- `REQ-003` is revised so FR-008 states that destroy removes the collection
  and all of its data (R-SDD-05).
- `PRAGMA foreign_keys = ON` on connection open is deferred to OBS-011 and is
  not a prerequisite of this contract.
- VACUUM or physical space reclamation is out of scope (story OQ-001).

## Traceability

- Source story: `US-016` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-010, DEC-015)