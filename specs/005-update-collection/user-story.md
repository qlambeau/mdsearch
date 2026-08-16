---
id: US-005
title: "Update a collection"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-002
feature: 005-update-collection
related:
  - US-004
  - US-001
  - US-002
  - US-003
---

# User Story

## Story Card

As a developer-curator,
I want to update a collection,
so that its stored files reflect the current on-disk state after files were
added, modified, or deleted.

## Context And Value

`US-004` ingests files but leaves the index stale when the vault changes. This
story reconciles a collection against the current filesystem: new files are
ingested, edited files are re-ingested, and removed files are dropped, keeping
the stored content ready for later indexing.

## Business Rules

- The command is `mdsearch collection update NAME PATH...`, or
  `mdsearch collection update --all`.
- `--database PATH` overrides the default database path
  `~/.mdsearch/collections.db`.
- `--force` switches unreadable paths from failing the command to being skipped
  and reported.
- `NAME` is validated like `collection create` and matched case-insensitively
  against an existing collection.
- Each `PATH` is a file or directory; directories are walked recursively, and
  only `.md` files are considered, consistent with `collection add`.
- The walked on-disk files are reconciled against the collection's stored files:
  - **added** — an on-disk file not yet stored is ingested;
  - **modified** — a stored file whose content hash differs is re-ingested;
  - **deleted** — a stored file whose path no longer exists on disk is removed;
  - **unchanged** — a stored file with the same path and hash is left as-is.
- Deleted detection applies to all stored files by path existence, not only the
  walked paths; a stored file that vanished from disk is removed.
- `--all` reconciles every collection's stored files, detecting modified and
  deleted files, without discovering new files.
- Without `--force`, an unreadable path fails the whole command and changes
  nothing; with `--force`, unreadable paths are skipped and the rest is applied.
- If the collection does not exist, the command fails and reports the collection
  was not found.
- If the selected database does not exist, the command fails and reports the
  database does not exist.
- Success output reports added, modified, and deleted counts; `--all` emits one
  line per collection.
- Exact human-readable wording of errors is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `Notes` stores `a.md` and `vault/` now also contains a new `b.md` | I run `mdsearch collection update Notes vault/` | `b.md` is ingested and the added count is reported |
| EX-002 | `Notes` stores `a.md` and `a.md` was edited on disk | I run `mdsearch collection update Notes a.md` | `a.md` is re-ingested and the modified count is reported |
| EX-003 | `Notes` stores `a.md` and `a.md` was deleted from disk | I run `mdsearch collection update Notes vault/` | `a.md` is removed and the deleted count is reported |
| EX-004 | `Notes` stores `a.md` unchanged | I run `mdsearch collection update Notes a.md` | `a.md` is left unchanged |
| EX-005 | Two collections store files, some edited and some deleted | I run `mdsearch collection update --all` | Each collection is reconciled and one line per collection is reported |
| EX-006 | A supplied path is unreadable | I run `mdsearch collection update Notes missing.md` | The command fails and changes nothing |
| EX-007 | A supplied path is unreadable | I run `mdsearch collection update Notes missing.md --force` | The path is skipped, the rest is applied, and the skip is reported |
| EX-008 | No collection named `Notes` exists | I run `mdsearch collection update Notes file.md` | The command fails and reports the collection was not found |
| EX-009 | No database exists at the selected path | I run `mdsearch collection update Notes file.md --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- Updating a collection ingests new on-disk `.md` files, re-ingests files whose
  content changed, and removes stored files whose path no longer exists.
- Unchanged files are left as-is.
- `--all` reconciles every collection's stored files.
- Without `--force`, an unreadable path fails the whole command and changes
  nothing.
- With `--force`, unreadable paths are skipped and reported.
- A missing collection or missing database fails semantically without creating
  a database file.
- `--database PATH` selects the database used by the operation.
- Success output reports added, modified, and deleted counts.

## Scope Boundaries

### In Scope

- Reconciling one or all collections against the current filesystem.
- Added, modified, and deleted detection using content hash and path existence.
- Atomic failure by default and `--force` skip-and-continue.

### Out Of Scope

- Adding files (that remains `collection add`).
- Lexical, semantic, or contextual indexing and search.
- Retrieving files or JSON output.
- Frontmatter parsing or content extraction.

## Dependencies

- `US-004` provides the `files` table, content hashing, and ingestion ports.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | None. | No | TBD | Resolved |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
