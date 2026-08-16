---
id: US-002
title: "List all collections"
type: user-story
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: PRD-001
epic: EPIC-001
feature: 002-list-collections
related:
  - US-001
---

# User Story

## Story Card

As a developer-curator,
I want to list all collections in a database,
so that I can see which collections exist before adding files, searching, or
destroying one.

## Context And Value

The list command reads the collection metadata already persisted by `US-001`
and renders the retained display names so a human can discover what exists. It
is read-only and performs no ingestion, indexing, or mutation. It is the first
command that makes the collections visible after creation, closing the gap left
by `US-001`, which created collections but offered no way to see them.

## Business Rules

- The command is `mdsearch collection list`.
- `--database PATH` overrides the default database path
  `~/.mdsearch/collections.db`.
- The command is read-only; it never creates, modifies, or initializes a
  database.
- Output is one collection display name per line.
- Names are sorted alphabetically, ignoring case.
- An existing database with no collections produces no output.
- If the selected database does not exist, the command fails and the output
  communicates that the database does not exist, without creating a database
  file.
- If the database exists but cannot be opened or read, the command fails and
  the output communicates that the database could not be accessed.
- Successful output contains no header, count, or decoration beyond the names.
- Exact human-readable wording of errors is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A database exists with collections `Notes` and `Archive` | I run `mdsearch collection list` | The output lists `Archive` then `Notes`, one per line |
| EX-002 | A database exists with collections `banana`, `Apple`, and `cherry` | I run `mdsearch collection list` | The output lists `Apple`, `banana`, and `cherry` in case-insensitive alphabetical order |
| EX-003 | A database exists with no collections | I run `mdsearch collection list` | The output is empty |
| EX-004 | No database exists at the selected path | I run `mdsearch collection list --database PATH` | The operation fails and reports that the database does not exist |
| EX-005 | The database exists but cannot be opened | I run `mdsearch collection list` | The operation fails and reports that the database could not be accessed |
| EX-006 | A collection was created in an earlier CLI run | I run `mdsearch collection list` in a later run | The collection still appears in the list |

## Acceptance Criteria

- Listing an existing database with collections prints each collection's
  retained display name, one per line, in case-insensitive alphabetical order.
- Listing an existing database with no collections produces no output.
- Listing a database that does not exist fails without creating a database file.
- Listing a database that cannot be opened fails and reports the access failure
  semantically.
- `--database PATH` selects the database listed.
- The command is read-only and performs no ingestion, indexing, or mutation.

## Scope Boundaries

### In Scope

- Listing all collections in the selected database.
- Case-insensitive alphabetical ordering.
- Read-only database access with failure reporting for missing or inaccessible
  databases.

### Out Of Scope

- Creating, destroying, or modifying collections.
- Adding files or indexing.
- Lexical, semantic, or contextual search.
- JSON or machine-readable output.

## Dependencies

- `US-001` collection creation must exist so collections can be persisted and
  listed.

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
