---
id: US-003
title: "Destroy a named collection"
type: user-story
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: PRD-001
epic: EPIC-001
feature: 003-destroy-collection
related:
  - US-001
  - US-002
---

# User Story

## Story Card

As a developer-curator,
I want to destroy a named collection,
so that I can remove collections I no longer need.

## Context And Value

Collection creation (`US-001`) and listing (`US-002`) let a curator build up and
inspect a set of collections. This story completes the collection lifecycle by
removing a named collection permanently, keeping the database tidy without
manual file editing.

## Business Rules

- The command is `mdsearch collection destroy NAME`.
- `--database PATH` overrides the default database path
  `~/.mdsearch/collections.db`.
- Collection-name matching is case-insensitive; the first-created spelling is
  destroyed.
- Destruction is permanent and requires no confirmation.
- Invalid names (empty or whitespace-only, path separators, or control
  characters) are rejected without touching the database, consistent with
  `collection create`.
- If the database exists but no collection matches the name, the command fails
  and reports that the collection was not found, leaving the database unchanged.
- If the selected database does not exist, the command fails and reports that
  the database does not exist.
- Destroying one collection leaves all other collections unchanged.
- A destroyed collection no longer appears in later `collection list` runs.
- Exact human-readable wording of errors is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A database contains `Notes` | I run `mdsearch collection destroy Notes` | `Notes` is removed and the output confirms its destruction |
| EX-002 | A database contains `Notes` | I run `mdsearch collection destroy notes` | The case-insensitive match removes `Notes` |
| EX-003 | A database contains `Notes` and `Archive` | I run `mdsearch collection destroy Notes` | `Archive` remains available |
| EX-004 | A database exists but has no collection matching `Missing` | I run `mdsearch collection destroy Missing` | The operation fails and reports the collection was not found, with no change |
| EX-005 | No database exists at the selected path | I run `mdsearch collection destroy Notes --database PATH` | The operation fails and reports the database does not exist |
| EX-006 | The name is empty, whitespace-only, or contains a path separator or control character | I run `mdsearch collection destroy` with that name | The operation is rejected and no collection is destroyed |
| EX-007 | `Notes` was destroyed in an earlier CLI run | I run `mdsearch collection list` | `Notes` no longer appears |

## Acceptance Criteria

- Destroying a matching collection permanently removes it.
- Name matching is case-insensitive.
- A successful destroy reports the destroyed collection in human-readable
  output.
- Destroying a non-existent collection in an existing database fails without
  changing the database.
- Destroying in a missing database fails without creating a database file.
- Invalid names are rejected without destroying anything.
- Other collections remain unchanged and listed after a destroy.
- `--database PATH` selects the database used by the operation.
- The command requires no confirmation.

## Scope Boundaries

### In Scope

- Destroying one named collection.
- Case-insensitive name matching.
- Semantic failure reporting for missing collections, missing databases, and
  invalid names.
- Persistence of the removal across CLI runs.

### Out Of Scope

- Listing, creating, or modifying collections.
- Adding files or indexing.
- Lexical, semantic, or contextual search.
- JSON or machine-readable output.
- Undo, soft delete, or recovery of a destroyed collection.

## Dependencies

- `US-001` collection creation must exist so collections can be persisted and
  destroyed.
- `US-002` listing makes the removal observable.

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
