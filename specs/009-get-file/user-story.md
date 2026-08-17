---
id: US-009
title: "Retrieve a complete file by name or ID"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-006
feature: 009-get-file
related:
  - US-004
  - US-006
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want to retrieve a complete stored file from a collection by name or ID,
so that I can fetch the full source content without searching.

## Context And Value

`US-004` stores ingested files with stable `file_id`s and their content, but
there is no way to pull a complete file back out. This story adds the `get`
command so a developer-curator or harness can retrieve a stored file's full
content by its canonical path, a unique basename, or its indexing-assigned ID.
It is the second slice of EPIC-006.

## Business Rules

- The command is `mdsearch get COLLECTION NAME_OR_ID`, with optional
  `--database PATH`.
- `COLLECTION` is matched case-insensitively against an existing collection.
- `NAME_OR_ID` is interpreted as follows:
  - A string of one or more decimal digits is treated as the indexing-assigned
    file ID.
  - Any other value is treated as a file name.
- A name matches a file in the collection if it equals the file's canonical
  absolute path, or if it equals the file's basename and that basename is unique
  within the collection.
- If a basename matches more than one file in the collection, the command fails
  and reports the ambiguity, listing the candidate paths.
- If no file in the collection matches the name or ID, the command fails and
  reports the file was not found.
- If the collection does not exist, the command fails and reports the collection
  was not found.
- If the selected database does not exist, the command fails and reports the
  database does not exist without creating a file.
- On success, the command prints the file's raw content to stdout exactly as
  stored, with no header, metadata, or decoration.
- Only files stored in the collection (via `collection add` or `collection
  update`) are retrievable.
- Exact human-readable wording of errors is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `Notes` stores `/vault/notes.md` with content "alpha" | I run `mdsearch get Notes /vault/notes.md` | The command prints "alpha" |
| EX-002 | `Notes` stores `/vault/notes.md` with a unique basename | I run `mdsearch get Notes notes.md` | The command prints the file's content |
| EX-003 | `Notes` stores a file with `file_id` 42 | I run `mdsearch get Notes 42` | The command prints that file's content |
| EX-004 | `Notes` stores `a/x.md` and `b/x.md` (duplicate basename) | I run `mdsearch get Notes x.md` | The command fails and lists both candidate paths |
| EX-005 | `Notes` stores no file named `missing.md` | I run `mdsearch get Notes missing.md` | The command fails and reports the file was not found |
| EX-006 | No file has `file_id` 999 in `Notes` | I run `mdsearch get Notes 999` | The command fails and reports the file was not found |
| EX-007 | No collection named `Journal` exists | I run `mdsearch get Journal notes.md` | The command fails and reports the collection was not found |
| EX-008 | No database exists at the selected path | I run `mdsearch get Notes notes.md --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- `mdsearch get COLLECTION NAME_OR_ID` retrieves a stored file by exact path, by
  unique basename, or by its positive-integer file ID.
- A duplicate basename fails with a clear error listing the candidate paths.
- A missing file, missing collection, or missing database fails with a clear
  error; a missing database does not create a file.
- On success, the raw file content is printed to stdout without decoration.

## Scope Boundaries

### In Scope

- Retrieving a stored file by exact path, unique basename, or file ID.
- Printing the complete raw content to stdout.
- Clear errors for ambiguity, not-found, missing collection, and missing database.

### Out Of Scope

- JSON output for retrieval (a later EPIC-006 consideration).
- Related-concept links (EPIC-006, dependent on EPIC-005).
- Case-insensitive or substring name matching.
- Retrieving files across collections in a single call.
- Answer generation or summarization.

## Dependencies

- `US-004` provides the `files` table with content, canonical paths, and stable
  `file_id`s.
- `US-006` confirms file IDs remain stable across updates.

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