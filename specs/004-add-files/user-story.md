---
id: US-004
title: "Add markdown files to a collection"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-002
feature: 004-add-files
related:
  - US-001
  - US-002
  - US-003
---

# User Story

## Story Card

As a developer-curator,
I want to add markdown files to a collection,
so that the collection has content that can later be indexed and searched.

## Context And Value

Collections are created empty (`US-001`). This story fills a collection with
markdown content, assigning each file a stable database identity and storing its
content and metadata. It is the first step of the ingestion pipeline that later
slices index lexically, semantically, and contextually.

## Business Rules

- The command is `mdsearch collection add NAME PATH...`.
- `--database PATH` overrides the default database path
  `~/.mdsearch/collections.db`.
- `--force` switches unreadable paths from failing the command to being skipped
  and reported.
- `NAME` is validated like `collection create` and matched case-insensitively
  against an existing collection.
- If the collection does not exist, the command fails and reports that the
  collection was not found.
- If the selected database does not exist, the command fails and reports that
  the database does not exist.
- Each `PATH` is a file or a directory; directories are walked recursively.
- Only `.md` files are ingested; non-`.md` files are ignored whether supplied
  directly or found under a directory.
- Files are identified by their canonical absolute path, so re-adding the same
  file from any directory updates it rather than duplicating it.
- Re-adding an already-ingested path replaces its stored content and metadata
  and retains its existing stable file ID.
- Each ingested file is assigned a stable file ID and stored with its content,
  content hash, byte size, and ingest timestamps.
- Without `--force`, if any supplied path, subdirectory, or file cannot be read,
  the whole command fails and ingests nothing.
- With `--force`, unreadable paths and files are skipped and the command
  continues, reporting how many were skipped.
- Success output reports the number of files added in human-readable form.
- Exact human-readable wording of errors is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A collection `Notes` exists and `vault/` contains `.md` files plus other files | I run `mdsearch collection add Notes vault/` | All `.md` files under `vault/` are ingested and the count is reported |
| EX-002 | A collection `Notes` exists and `notes.md` is a readable `.md` file | I run `mdsearch collection add Notes notes.md` | `notes.md` is ingested with a stable file ID |
| EX-003 | `notes.md` was already ingested | I run `mdsearch collection add Notes notes.md` again | The stored content is replaced and no duplicate file is created |
| EX-004 | A collection `Notes` exists but `missing.md` does not exist | I run `mdsearch collection add Notes missing.md` | The command fails and ingests nothing |
| EX-005 | `missing.md` does not exist | I run `mdsearch collection add Notes missing.md --force` | `missing.md` is skipped, the rest is ingested, and the skip is reported |
| EX-006 | No collection named `Notes` exists | I run `mdsearch collection add Notes file.md` | The command fails and reports the collection was not found |
| EX-007 | No database exists at the selected path | I run `mdsearch collection add Notes file.md --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- Adding files to an existing collection ingests every `.md` file reachable
  through the supplied paths, recursively for directories.
- Non-`.md` files are ignored.
- Each ingested file is stored with a stable file ID, content, hash, byte size,
  and timestamps.
- Re-adding an already-ingested path replaces its content without creating a
  duplicate.
- Without `--force`, an unreadable path fails the whole command and ingests
  nothing.
- With `--force`, unreadable paths are skipped, the rest is ingested, and the
  skip count is reported.
- A missing collection or missing database fails semantically without creating
  a database file.
- `--database PATH` selects the database used by the operation.
- Success output reports the number of files added.

## Scope Boundaries

### In Scope

- Recursively ingesting `.md` files from file and directory paths.
- Stable file IDs and persisted file content and metadata.
- Upsert semantics for re-added paths.
- Atomic failure by default and `--force` skip-and-continue.

### Out Of Scope

- The `update` command that reconciles added, modified, and deleted files.
- Lexical, semantic, or contextual indexing and search.
- Retrieving files or JSON output.
- Frontmatter parsing or content extraction.

## Dependencies

- `US-001` collection creation must exist so collections can hold files.
- Content hashing requires an approved crate (`sha2`), recorded in `DES-004`.

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
