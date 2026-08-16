---
id: REQ-005
title: "Update a collection requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-005
related:
  - US-004
  - US-001
  - US-002
  - US-003
  - DES-005
---

# Requirements

## Purpose And Actors

### Purpose

Allow a developer-curator to reconcile a collection's stored files with the
current on-disk state, ingesting new files, re-ingesting modified files, and
removing deleted files.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local filesystem containing the markdown files.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch collection update NAME PATH...` or
  `mdsearch collection update --all`.
- `NAME` matches an existing collection case-insensitively.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Update a collection | `NAME`, one or more `PATH`, optional `--database PATH`, optional `--force` | Human-readable added/modified/deleted counts, plus skipped count under `--force` | `NAME` matched case-insensitively; only `.md` files considered |
| Update all collections | `--all`, optional `--database PATH` | One human-readable line per collection | None beyond the database |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The CLI shall accept `mdsearch collection update NAME PATH...` to update one collection. | Must | US-005; Update ingests newly added files |
| FR-002 | The CLI shall accept `mdsearch collection update --all` to update every collection. | Must | US-005; Update all collections |
| FR-003 | The CLI shall use `~/.mdsearch/collections.db` as the database path when no override is supplied. | Must | US-005; Update ingests newly added files |
| FR-004 | The CLI shall use the path supplied by `--database PATH` instead of the default path. | Must | US-005; Report a missing database |
| FR-005 | The CLI shall walk each `PATH` recursively and consider only `.md` files. | Must | US-005; Update ingests newly added files |
| FR-006 | The CLI shall ingest an on-disk `.md` file that is not yet stored. | Must | US-005; Update ingests newly added files |
| FR-007 | The CLI shall re-ingest a stored file whose content hash differs from disk. | Must | US-005; Update re-ingests a modified file |
| FR-008 | The CLI shall remove a stored file whose path no longer exists on disk. | Must | US-005; Update removes a deleted file |
| FR-009 | The CLI shall leave unchanged a stored file with the same path and hash. | Must | US-005; Update leaves an unchanged file as-is |
| FR-010 | `--all` shall detect modified and deleted files for every collection without discovering new files. | Must | US-005; Update all collections |
| FR-011 | Without `--force`, an unreadable path shall fail the command and change nothing. | Must | US-005; Fail without changing anything when a path is unreadable |
| FR-012 | With `--force`, unreadable paths shall be skipped and the rest applied. | Must | US-005; Skip unreadable paths with --force |
| FR-013 | A missing collection shall fail semantically without creating a database file. | Must | US-005; Report a missing collection |
| FR-014 | A missing database shall fail semantically without creating a database file. | Must | US-005; Report a missing database |
| FR-015 | Success output shall report added, modified, and deleted counts. | Must | US-005; Update ingests newly added files |

## Postconditions And Invariants

- After a successful update, every stored file matches its on-disk state for
  the walked scope: added files are stored, modified files are refreshed, and
  deleted files are removed.
- A failed update without `--force` changes nothing.
- File IDs remain stable across modifications of the same path.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| A supplied path does not exist or cannot be read | Fail without change, or skip under `--force` | Failure names the unreadable path; `--force` reports it skipped |
| A stored file's path no longer exists | Remove it | Deleted count increments |
| `NAME` does not match a collection | Fail without change | Output communicates the collection was not found |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |
| `--database PATH` is supplied | Use `PATH` rather than the default path | Success or failure applies to the selected database |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for this command.
- The operation shall not perform lexical, semantic, or entity indexing, nor
  generate answers.

## Traceability

- Source story: `US-005` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
