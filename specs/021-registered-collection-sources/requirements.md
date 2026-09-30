---
id: REQ-021
title: "Registered collection sources and source-aware updates"
type: feature-requirements
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved REQ-021 in chat on 2026-09-30"
parent: US-021
depends_on: []
requires: []
blockers: []
related:
  - US-005
  - US-006
  - US-012
  - ADR-005
---

# Requirements

## Purpose And Actors

### Purpose

Persist each collection's Markdown sources so explicit updates discover all
current files, reconcile stored content, and keep lexical and graph indexes
consistent without requiring source paths to be repeated.

### Actors And External Systems

- Developer-curators managing local Markdown collections.
- The local filesystem containing source files and directories.
- The local SQLite collection database.
- Coding-agent harnesses consuming collection-list JSON output.

## Preconditions

- The selected database is available at the configured path.
- A collection targeted by configure or update exists.
- Source arguments identify Markdown files or directories. Relative paths are
  resolved from the invoking working directory and represented in canonical
  form in persisted and displayed source data.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Create collection with sources | Collection name and one or more file/directory paths | Collection and its registered sources are stored; no files are indexed | Collection name follows existing name rules; source paths resolve to canonical paths |
| Configure sources | Collection name and one or more replacement paths | The collection's previous source list is replaced | Collection exists; source paths resolve to canonical paths |
| List collections | Optional `--json` | Human output shows collection names and sources; JSON is `{"collections":[{"name":"Notes","sources":["/canonical/path"]}]}` | JSON includes an empty `collections` array when none exist; a source-less collection has an empty `sources` array |
| Update one collection | Collection name and optional `--skip-unreadable` | Per-collection added/modified/deleted/skipped outcome | A collection without registered sources fails with registration guidance |
| Update all collections | `--all` and optional `--skip-unreadable` | One outcome per collection | Each collection is processed independently; any collection failure makes the command fail after all are attempted |

## Functional Requirements

| ID | Requirement | Priority | Traceability (Story & Scenario) |
| --- | --- | --- | --- |
| FR-001 | Creating a collection with source paths shall persist their canonical forms without ingesting files or building indexes. | Must | US-021 / Creating a collection registers sources without indexing |
| FR-002 | Configuring a collection with `--sources PATH...` shall replace its registered source set. | Must | US-021 / Configuring sources replaces the registered source set |
| FR-003 | Collection listing shall show each collection's registered canonical source paths in human-readable output. With `--json`, output shall contain a `collections` array of objects with `name` and `sources` fields. | Must | US-021 / Collection listing reports canonical registered sources |
| FR-004 | A registered Markdown file shall be considered directly; a registered directory shall be scanned recursively for `.md` files. | Must | US-021 / Update discovers new Markdown files; Update reconciles edits and deletions from registered sources |
| FR-005 | A file discovered through overlapping registered sources shall be processed once per collection update. | Must | US-021 / Overlapping sources process a file once |
| FR-006 | Updating a collection shall add newly discovered files, refresh changed files, and remove files that no longer exist in any registered source. | Must | US-021 / Update discovers new Markdown files; Update reconciles edits and deletions from registered sources |
| FR-007 | After a source is removed, the next successful update shall remove files found only through that source and retain files still found through a remaining source. | Must | US-021 / Removing a source deletes files exclusive to that source |
| FR-008 | An inaccessible directory source shall fail that collection's update without changing its stored content or indexes. | Must | US-021 / An inaccessible directory aborts the collection update |
| FR-009 | With `--skip-unreadable`, an unreadable individual file shall be reported as skipped; if it already has stored content, that content shall remain unchanged. | Must | US-021 / Skipping an unreadable file preserves its previous content |
| FR-010 | For each collection, file reconciliation and lexical/graph index rebuilding shall commit together or leave the previously committed files and indexes unchanged. | Must | US-021 / Update commits content and indexes atomically |
| FR-011 | `update --all` shall attempt every collection independently, report each collection's outcome, and return a failure status if any collection fails. | Must | US-021 / Update all continues after an individual collection failure |
| FR-012 | A migrated collection without registered sources shall remain searchable, shall not receive guessed source paths, and shall fail update with guidance to register sources. | Must | US-021 / A migrated collection is not assigned guessed sources |
| FR-013 | Registering or replacing sources alone shall not ingest content or build indexes. | Must | US-021 / Creating a collection registers sources without indexing; Configuring sources replaces the registered source set |

## Postconditions And Invariants

- A collection's persisted source set is the set used by later updates and
  source-aware collection listing.
- Each file path is reconciled at most once per update even when roots overlap.
- A successful update's stored content and lexical/graph indexes describe the
  same discovered file set.
- A failed collection update leaves its previously committed content and
  lexical/graph indexes unchanged.
- A failed collection in an update-all run does not roll back successful updates
  to other collections.
- Existing databases are migrated without inferring source paths from stored
  file paths.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Registered directory cannot be enumerated | Fail the affected collection update before commit | Error identifies the collection/source problem; its previous content and indexes remain unchanged |
| Individual file is unreadable and skipping is enabled | Skip that file and preserve any prior stored content | The outcome reports the skipped file/count |
| No source paths are registered for a migrated collection | Do not guess sources or mutate stored content | Update fails with guidance to configure sources |
| Lexical or graph rebuild fails | Roll back file and index changes for that collection | Update reports failure; previous committed state remains available |
| One collection fails during update-all | Continue with other collections | Per-collection outcomes are reported and the overall command exits unsuccessfully |
| A source is removed but its file is present through another registered source | Keep the file | The file remains stored and indexed once |

## Quality Requirements

- Updates remain explicit and local; no background watcher or network access is
  introduced.
- Collection updates are atomic at collection scope and do not require a
  database-wide transaction across `update --all`.
- Path overlap and ordering do not produce duplicate files or duplicate index
  entries.
- JSON source-list output is deterministic for a fixed database state.

## Traceability

- Source story: `US-021` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001`, EPIC-002 in `specs/prds/PRD-001.md`
