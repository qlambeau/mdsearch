---
id: REQ-002
title: "List all collections requirements"
type: feature-requirements
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: US-002
related:
  - US-001
  - DES-002
---

# Requirements

## Purpose And Actors

### Purpose

Allow a developer-curator to list all collections stored in a database, so they
can discover which collections exist before adding files, searching, or
destroying one.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch collection list`.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- The selected database may exist with zero or more collections, or may not
  exist at all.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| List collections | Optional `--database PATH` | Each collection's retained display name, one per line, in case-insensitive alphabetical order | Names are read as stored; no input to validate beyond the optional path |
| List an empty database | An existing database with no collections | No output | None |
| List a missing database | A path to a database that does not exist | A failure communicating that the database does not exist | Do not create a database file |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The CLI shall accept `mdsearch collection list` to request a listing of all collections. | Must | US-002; List collections in case-insensitive alphabetical order |
| FR-002 | The CLI shall use `~/.mdsearch/collections.db` as the database path when no override is supplied. | Must | US-002; List collections in case-insensitive alphabetical order |
| FR-003 | The CLI shall use the path supplied by `--database PATH` instead of the default path. | Must | US-002; List a database that does not exist; List a collection created in an earlier CLI run |
| FR-004 | The CLI shall print each collection's retained display name, one per line, in case-insensitive alphabetical order, with no header, count, or other decoration. | Must | US-002; List collections in case-insensitive alphabetical order; List collections ignoring letter case in the sort order |
| FR-005 | The CLI shall produce no output when the selected database exists and contains no collections. | Must | US-002; List an existing database with no collections |
| FR-006 | If the selected database does not exist, the CLI shall fail, communicate that the database does not exist, and not create a database file. | Must | US-002; List a database that does not exist |
| FR-007 | If the selected database cannot be opened or read, the CLI shall fail and communicate that the database could not be accessed. | Must | US-002; List a database that cannot be opened |
| FR-008 | The list operation shall be read-only and shall never create, modify, or initialize a database. | Must | US-002; List a database that does not exist |
| FR-009 | A collection created in an earlier CLI run shall remain listed in a later run against the same database. | Must | US-002; List a collection created in an earlier CLI run |

## Postconditions And Invariants

- A successful list changes nothing in the selected database.
- A failed list leaves no new or changed database file.
- Output is the stored display names in case-insensitive alphabetical order,
  one per line, and nothing else.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Database does not exist at the selected path | Fail without creating a database file | Output communicates that the database does not exist |
| Database exists but cannot be opened or read | Fail without modifying the database | Output communicates that the database could not be accessed |
| Database exists with no collections | Succeed with no output | Empty output |
| `--database PATH` is supplied | Use `PATH` rather than the default path | Success or failure applies to the selected database |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for this command.
- The operation shall not add files, perform indexing, or generate answers.

## Traceability

- Source story: `US-002` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
