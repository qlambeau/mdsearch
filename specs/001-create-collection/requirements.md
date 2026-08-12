---
id: REQ-001
title: "Create an empty named collection requirements"
type: feature-requirements
status: implemented
created: 2026-08-11
updated: 2026-08-11
owner: TBD
parent: US-001
related: []
---

# Requirements

## Purpose And Actors

### Purpose

Allow a developer-curator to create one empty, named collection that is stored
locally and remains available for later file ingestion and indexing.

### Actors And External Systems

- Developer-curator invoking the `kv` CLI.
- Local filesystem containing the collection database.
- The local collection database addressed by the default path or an explicit path override.

## Preconditions

- The user invokes `kv collection create NAME`.
- The collection name is supplied as the command's `NAME` argument.
- The database path is `~/.kv/collections.db` unless `--database PATH` is supplied.
- The database may already exist or may need to be initialized.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Create a collection | Required `NAME`; optional `--database PATH` | Human-readable success confirmation containing the retained collection name; an empty persistent collection | Trim surrounding whitespace; reject empty or whitespace-only values, path separators, control characters, and case-insensitive duplicates; no product-defined maximum length |
| Initialize the default database | No existing database at `~/.kv/collections.db` | A usable local database containing the requested collection | Fail without a partial collection if the database cannot be created or opened |
| Initialize or open an overridden database | `--database PATH` | A usable local database at `PATH` containing the requested collection | Use the supplied path instead of the default; fail without a partial collection if it cannot be created or opened |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The CLI shall accept `kv collection create NAME` to request creation of one collection. | Must | US-001; Initialize the default database and create the first collection |
| FR-002 | The CLI shall use `~/.kv/collections.db` as the database path when no override is supplied. | Must | US-001; Initialize the default database and create the first collection |
| FR-003 | The CLI shall use the path supplied by `--database PATH` instead of the default path. | Must | US-001; Create a collection in an explicitly selected database |
| FR-004 | The CLI shall trim surrounding whitespace from the requested collection name before validation and storage. | Must | US-001; Trim surrounding whitespace before storing a collection name |
| FR-005 | The CLI shall reject a collection name that is empty, whitespace-only, contains a path separator, or contains a control character. | Must | US-001; Reject an invalid collection name |
| FR-006 | The CLI shall treat collection names as case-insensitively unique while retaining the first-created spelling for display. | Must | US-001; Reject a case-insensitive duplicate collection name |
| FR-007 | When the selected database does not exist, the CLI shall initialize it and create the requested empty collection. | Must | US-001; Initialize the default database and create the first collection; Create a collection in an explicitly selected database |
| FR-008 | On successful creation, the CLI shall create exactly one empty collection and produce human-readable output confirming the retained collection name. | Must | US-001; Initialize the default database and create the first collection; Create a collection in an explicitly selected database |
| FR-009 | A successfully created collection shall remain available when the CLI is run again against the same database. | Must | US-001; Preserve a collection across CLI runs |
| FR-010 | The CLI shall reject a case-insensitive duplicate without modifying or replacing the existing collection. | Must | US-001; Reject a case-insensitive duplicate collection name |
| FR-011 | If the selected database cannot be created or opened, the CLI shall fail and shall not leave a partial collection. | Must | US-001; Fail without a partial collection when the database is inaccessible |
| FR-012 | Failure output shall communicate the relevant failure semantically; exact human-readable wording is not part of this contract. | Must | US-001; Reject a case-insensitive duplicate collection name; Reject an invalid collection name; Fail without a partial collection when the database is inaccessible |

## Postconditions And Invariants

- A successful operation leaves exactly one new empty collection associated with the selected database.
- The stored display name is the trimmed spelling from the first successful creation.
- No two collections in the same database have names that are equal under case-insensitive comparison.
- A successful collection remains available after the creating CLI process exits.
- A rejected or failed operation does not create, replace, or modify a collection.
- Database initialization and collection creation are treated as one operation from the user's perspective; failure must not leave partial collection state.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Database does not exist at the selected path | Initialize the database and continue creation | Success confirmation names the new collection |
| `NAME` is empty or whitespace-only | Reject before creating a collection | Output communicates that the collection name is invalid |
| `NAME` contains a path separator or control character | Reject before creating a collection | Output communicates that the collection name is invalid |
| A case-insensitive equivalent already exists | Reject without changing the existing collection | Output communicates that the name is already in use |
| Database cannot be created or opened | Fail without leaving partial collection state | Output communicates that the database could not be accessed |
| `--database PATH` is supplied | Use `PATH` rather than the default path | Success or failure applies to the selected database |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Collection creation and database initialization shall preserve the invariants above even when the operation fails.
- Human-readable output shall be the default output form for this command.
- The operation shall not add files, perform indexing, or generate answers.

## Dependencies And Resolved Decisions

- `ADR-001` selects Rust with SQLite and the `sqlite-vector` extension as the embedded storage foundation.
- The database-engine choice does not change the external behavior defined here.

## Traceability

- Source story: `US-001` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
