---
id: REQ-004
title: "Add markdown files to a collection requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-004
related:
  - US-001
  - US-002
  - US-003
  - DES-004
---

# Requirements

## Purpose And Actors

### Purpose

Allow a developer-curator to add markdown files to an existing collection,
storing each file's content and metadata with a stable database identity for
later indexing and retrieval.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local filesystem containing the markdown files.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch collection add NAME PATH...`.
- `NAME` matches an existing collection case-insensitively.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- Each `PATH` is a file or directory on the local filesystem.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Add files | `NAME`, one or more `PATH`, optional `--database PATH`, optional `--force` | Human-readable count of files added, plus skipped count under `--force` | `NAME` validated and matched case-insensitively; only `.md` files are ingested |
| Add a directory | A directory `PATH` | All reachable `.md` files ingested recursively | Non-`.md` files ignored |
| Add an unreadable path | A path that cannot be read | Without `--force`: failure with nothing ingested; with `--force`: skip and continue | Detect missing or unreadable paths |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The CLI shall accept `mdsearch collection add NAME PATH...` to add files to one collection. | Must | US-004; Add markdown files from a directory recursively |
| FR-002 | The CLI shall use `~/.mdsearch/collections.db` as the database path when no override is supplied. | Must | US-004; Add a single markdown file |
| FR-003 | The CLI shall use the path supplied by `--database PATH` instead of the default path. | Must | US-004; Add files to an explicitly selected database |
| FR-004 | The CLI shall accept a `--force` switch that changes unreadable-path handling from failure to skip-and-continue. | Must | US-004; Skip unreadable paths with --force |
| FR-005 | The CLI shall match `NAME` case-insensitively and fail, reporting the collection was not found, when no matching collection exists. | Must | US-004; Report a missing collection |
| FR-006 | If the selected database does not exist, the CLI shall fail, report that the database does not exist, and not create a database file. | Must | US-004; Report a missing database |
| FR-007 | The CLI shall treat each `PATH` as a file or directory and walk directories recursively. | Must | US-004; Add markdown files from a directory recursively |
| FR-008 | The CLI shall ingest only `.md` files and ignore non-`.md` files whether supplied directly or found under a directory. | Must | US-004; Add markdown files from a directory recursively |
| FR-009 | The CLI shall identify files by their canonical absolute path and replace the stored content and metadata when an already-ingested path is added again. | Must | US-004; Re-adding a file replaces its content without duplicating |
| FR-010 | The CLI shall store each ingested file's content, content hash, byte size, and ingest timestamps under a database-assigned stable file ID. | Must | US-004; Add a single markdown file |
| FR-011 | Without `--force`, if any supplied path, subdirectory, or file cannot be read, the CLI shall fail and ingest nothing. | Must | US-004; Fail without ingesting when a path is unreadable |
| FR-012 | With `--force`, the CLI shall skip unreadable paths, ingest the rest, and report the number skipped. | Must | US-004; Skip unreadable paths with --force |
| FR-013 | On success, the CLI shall report the number of files added in human-readable output. | Must | US-004; Add a single markdown file |
| FR-014 | Re-adding an already-ingested path shall retain its existing stable file ID. | Must | US-004; Re-adding a file replaces its content without duplicating |

## Postconditions And Invariants

- A successful add leaves each reachable `.md` file stored exactly once under its
  canonical absolute path.
- No two ingested files in the same collection share a canonical path.
- File IDs are stable across re-adds of the same path.
- Without `--force`, a failed add ingests nothing; the collection's file set is
  unchanged.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| A supplied path does not exist or cannot be read | Fail without ingesting, or skip under `--force` | Failure names the unreadable path; `--force` reports it as skipped |
| A directory contains no `.md` files | Succeed with zero files added | Output reports zero files added |
| A non-`.md` file is supplied directly | Ignore it | It is not ingested |
| `NAME` does not match a collection | Fail without change | Output communicates the collection was not found |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |
| `--database PATH` is supplied | Use `PATH` rather than the default path | Success or failure applies to the selected database |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for this command.
- The operation shall not perform lexical, semantic, or entity indexing, nor
  generate answers.

## Traceability

- Source story: `US-004` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
