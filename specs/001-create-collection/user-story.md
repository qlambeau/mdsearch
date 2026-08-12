---
id: US-001
title: "Create an empty named collection"
type: user-story
status: implemented
created: 2026-08-11
updated: 2026-08-11
owner: TBD
parent: PRD-001
epic: EPIC-001
feature: 001-create-collection
related: []
---

# User Story

## Story Card

As a developer-curator,
I want to create a named collection,
so that I can organize markdown files for later indexing and retrieval.

## Context And Value

A collection is the user-managed boundary for a group of markdown files. The
collection must exist independently of its files so that ingestion and indexing
can be performed later through EPIC-002.

## Business Rules

- A collection starts empty.
- Names are trimmed before validation.
- Empty or whitespace-only names are rejected.
- Names containing path separators or control characters are rejected.
- There is no product-defined maximum name length.
- Name uniqueness is case-insensitive.
- The first-created spelling is retained for display.
- Creating a duplicate name is rejected without changing the existing collection.
- If no database exists, it is initialized automatically.
- The default database path is `~/.kv/collections.db`.
- `--database PATH` overrides the default database path.
- The collection persists across CLI runs.
- If the database cannot be created or opened, the operation fails without creating a partial collection.
- The command is `kv collection create NAME`.
- Successful creation produces a human-readable confirmation containing the collection name.
- Error output must communicate the relevant failure semantically, but exact wording is not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | No database exists and `Notes` is an unused valid name | I run `kv collection create Notes` | The database and empty `Notes` collection are created, and the name is shown in a confirmation |
| EX-002 | A custom database path is supplied and `Project Notes` is unused | I run `kv collection create "Project Notes" --database PATH` | The empty collection is created in the specified database |
| EX-003 | A collection named `Notes` already exists | I run `kv collection create notes` | The operation is rejected and the existing collection is unchanged |
| EX-004 | The name is empty, whitespace-only, or contains a path separator or control character | I run the create command | The operation is rejected and no collection is created |
| EX-005 | The database cannot be created or opened | I run the create command with a valid name | The operation fails without leaving a partial collection |
| EX-006 | A collection was created in an earlier CLI run | I run the CLI again | The collection remains available |

## Acceptance Criteria

- A valid, unused name creates one empty collection.
- The first collection creation initializes the database when it does not exist.
- `--database PATH` selects the database used by the operation.
- A created collection remains available in later CLI runs.
- Collection-name comparison is case-insensitive while the first-created spelling is retained.
- A duplicate name is rejected without changing the existing collection.
- Invalid names are rejected without creating a collection.
- Database access failures are reported without leaving a partial collection.
- Successful creation reports the retained collection name in human-readable output.

## Scope Boundaries

### In Scope

- Creating one empty named collection.
- Name validation and case-insensitive uniqueness.
- Database initialization and location override.
- Persistence across CLI runs.
- Human-readable success and semantic failure behavior.

### Out Of Scope

- Listing collections.
- Destroying collections.
- Adding files or indexing.
- Lexical, semantic, or contextual search.
- File retrieval and retrieval-specific JSON output.

## Dependencies

- The embedded database decision must be resolved during technical design.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Which embedded database engine meets the product's single-file collection and search constraints? | No | TBD | Resolved in ADR-001 |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
