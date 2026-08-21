---
id: REQ-003
title: "Destroy a named collection requirements"
type: feature-requirements
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: US-003
related:
  - US-001
  - US-002
  - DES-003
---

# Requirements

## Purpose And Actors

### Purpose

Allow a developer-curator to permanently destroy one named collection, removing
it from the selected database.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch collection destroy NAME`.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- The selected database may exist with zero or more collections, or may not
  exist at all.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Destroy a collection | Required `NAME`; optional `--database PATH` | Human-readable confirmation naming the destroyed collection; the collection is removed | Trim whitespace; reject empty, whitespace-only, path-separator, and control-character names; match case-insensitively |
| Destroy a missing collection | `NAME` with no matching collection | A failure communicating the collection was not found | Leave the database unchanged |
| Destroy in a missing database | A path to a database that does not exist | A failure communicating that the database does not exist | Do not create a database file |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The CLI shall accept `mdsearch collection destroy NAME` to request destruction of one collection. | Must | US-003; Destroy a collection by name |
| FR-002 | The CLI shall use `~/.mdsearch/collections.db` as the database path when no override is supplied. | Must | US-003; Destroy a collection by name |
| FR-003 | The CLI shall use the path supplied by `--database PATH` instead of the default path. | Must | US-003; Destroy in a database that does not exist |
| FR-004 | The CLI shall match the requested name case-insensitively and destroy the matching collection. | Must | US-003; Destroy a collection case-insensitively |
| FR-005 | The CLI shall reject an invalid name (empty, whitespace-only, path separator, or control character) without destroying any collection. | Must | US-003; Reject an invalid collection name |
| FR-006 | If the database exists but no collection matches the name, the CLI shall fail, report that the collection was not found, and leave the database unchanged. | Must | US-003; Destroy a non-existent collection in an existing database |
| FR-007 | If the selected database does not exist, the CLI shall fail, report that the database does not exist, and not create a database file. | Must | US-003; Destroy in a database that does not exist |
| FR-008 | On success, the CLI shall permanently remove the collection and produce human-readable output confirming the destroyed collection name. | Must | US-003; Destroy a collection by name |
| FR-009 | Destroying one collection shall leave all other collections unchanged. | Must | US-003; Destroy one collection without disturbing others |
| FR-010 | A destroyed collection shall no longer appear in a later `collection list` run. | Must | US-003; A destroyed collection no longer appears in a later listing |
| FR-011 | The destroy command shall require no confirmation. | Must | US-003; Destroy a collection by name |
| FR-012 | On success, the destroy shall remove the collection and every row belonging to it — stored files, FTS5 passages, passage mappings, vectors, graph nodes and edges, and all index-state rows — in one atomic transaction; a failure at any point rolls back and leaves the collection and its data intact. | Must | US-016; Destroying a fully indexed collection removes every trace; A failed destroy leaves the collection intact |

## Postconditions And Invariants

- A successful destroy removes exactly the matching collection and every row
  belonging to it in all per-collection tables.
- Destroying a collection does not change any other collection.
- A failed destroy leaves the database unchanged and creates no database file.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Database does not exist at the selected path | Fail without creating a database file | Output communicates that the database does not exist |
| No collection matches the requested name | Fail without changing the database | Output communicates that the collection was not found |
| A storage failure occurs during the destroy | Roll back the whole delete | The collection and its data remain intact; a storage error is reported |
| `NAME` is empty, whitespace-only, or contains a path separator or control character | Reject before touching the database | Output communicates that the collection name is invalid |
| `--database PATH` is supplied | Use `PATH` rather than the default path | Success or failure applies to the selected database |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for this command.
- The operation shall not add files, perform indexing, or generate answers.

## Traceability

- Source story: `US-003` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
