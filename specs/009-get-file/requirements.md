---
id: REQ-009
title: "Retrieve a complete file by name or ID requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-009
related:
  - US-004
  - US-006
  - REQ-004
  - DB-001
  - TABLE-003
---

# Requirements

## Purpose And Actors

### Purpose

Provide a command that retrieves a complete stored file from a collection by its
canonical path, a unique basename, or its indexing-assigned ID, printing the raw
content for a human or coding-agent harness.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch get COLLECTION NAME_OR_ID` with optional
  `--database PATH`.
- `COLLECTION` names an existing collection.
- A file with the given name or ID is stored in that collection.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Retrieve a file | `COLLECTION`, `NAME_OR_ID`, optional `--database PATH` | The stored file's raw content | `COLLECTION` matched case-insensitively; `NAME_OR_ID` resolved to a unique file |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `mdsearch get COLLECTION NAME_OR_ID` shall retrieve a stored file, with `--database PATH` selecting the database. | Must | US-009; Retrieve a file by its canonical path |
| FR-002 | `COLLECTION` shall be matched case-insensitively, and an unknown collection shall fail and report the collection was not found. | Must | US-009; Report a missing collection |
| FR-003 | An all-digit positive argument shall be treated as the indexing-assigned file ID; any other value shall be treated as a name. | Must | US-009; Retrieve a file by its indexing-assigned ID |
| FR-004 | A name shall match a file whose canonical path equals the name, or whose basename equals the name when that basename is unique in the collection. | Must | US-009; Retrieve a file by its canonical path; Retrieve a file by a unique basename |
| FR-005 | A basename matching more than one file shall fail and report the ambiguity, listing the candidate paths. | Must | US-009; Report an ambiguous basename with candidates |
| FR-006 | No file matching the name or ID shall fail and report the file was not found. | Must | US-009; Report a file not found by name; Report a file not found by ID |
| FR-007 | A missing database shall fail without creating a file. | Must | US-009; Report a missing database without creating a file |
| FR-008 | On success, the command shall print the file's raw content to stdout with no header, metadata, or decoration. | Must | US-009; Retrieve a file by its canonical path |

## Postconditions And Invariants

- A successful retrieval prints the exact stored content.
- Retrieval mutates nothing in the database.
- Only files stored in the collection (via `collection add` or `update`) are
  retrievable.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| A basename matches more than one file | Fail | Output lists the candidate paths |
| No file matches the name or ID | Fail | Output communicates the file was not found |
| The collection does not exist | Fail | Output communicates the collection was not found |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |

## Quality Requirements

- The operation shall work offline and remain read-only.
- The retrieved content shall be the exact stored bytes.
- Human-readable raw output shall be the default; no JSON is produced in this
  slice.

## Traceability

- Source story: `US-009` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`