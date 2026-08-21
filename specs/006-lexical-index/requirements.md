---
id: REQ-006
title: "Build the lexical index during collection update requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-006
related:
  - US-004
  - US-005
  - DES-004
  - DES-005
  - ADR-001
  - TABLE-003
  - DB-001
---

# Requirements

## Purpose And Actors

### Purpose

Make `collection update` build and keep current a per-passage lexical (BM25)
index over the collection's stored files, and provide `mdsearch index status`
so a developer-curator can observe whether the index is built and how fresh it
is, before the search command is added in a later EPIC-003 slice.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local filesystem containing the markdown files.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch collection update NAME PATH...`,
  `mdsearch collection update --all`, or `mdsearch index status`.
- `NAME` matches an existing collection case-insensitively.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Update a collection | `NAME`, one or more `PATH`, optional `--database PATH`, optional `--force` | Human-readable added/modified/deleted counts, plus skipped count under `--force`; the lexical index is rebuilt in the same transaction | `NAME` matched case-insensitively; only `.md` files considered |
| Update all collections | `--all`, optional `--database PATH` | One human-readable line per collection; each collection's index is rebuilt | None beyond the database |
| Index status | No collection name, optional `--database PATH` | One human-readable line per collection with index state, file count, passage count, and last-build timestamp; empty output for a database with no collections | None beyond the database |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `collection update` shall rebuild the lexical index for each affected collection in the same transaction as the file reconciliation. | Must | US-006; Update builds the index and counts passages |
| FR-002 | `collection add` alone shall not build the lexical index. | Must | US-006; Adding files alone does not build the index |
| FR-003 | The lexical index shall segment each file's body into passages by splitting on one or more blank lines, with each paragraph indexed as one passage. | Must | US-006; Update builds the index and counts passages |
| FR-004 | The recognized frontmatter fields `title`, `tags`, `aliases`, and `summary` shall each be indexed as their own passage. | Must | US-006; Every recognized frontmatter field becomes its own passage |
| FR-005 | A file without frontmatter shall be indexed body-only. | Must | US-006; Files without frontmatter are indexed by their body |
| FR-006 | A file with malformed or unparseable frontmatter shall be indexed body-only and reported, without failing the update. | Must | US-006; Malformed frontmatter is indexed body-only and reported |
| FR-007 | An empty file with no paragraphs and no frontmatter fields shall contribute no passages and shall not fail the update. | Must | US-006; Empty files contribute no passages |
| FR-008 | The lexical index shall be refreshed by an update so that edited files contribute updated passages and deleted files contribute none. | Must | US-006; Update refreshes the index after an edit; Update removes passages of a deleted file |
| FR-009 | `mdsearch index status` shall report, for every collection, the lexical index state (`built` or `not built`), the stored file count, the indexed passage count, and the last-build timestamp. | Must | US-006; Update builds the index and counts passages |
| FR-010 | A collection shall show state `not built` until an update has built its index. | Must | US-006; Adding files alone does not build the index |
| FR-011 | A collection with zero indexed passages shall still show state `built` after a successful update. | Must | US-006; Empty files contribute no passages |
| FR-012 | `mdsearch index status` shall accept no collection name and report all collections. | Must | US-006; Update --all rebuilds the index for every collection |
| FR-013 | If the index build fails, the whole update shall fail, no file changes shall be committed, and the previous index state shall be unchanged. | Must | US-006; Index build failure fails the update atomically |
| FR-014 | `mdsearch index status` against a missing database shall fail semantically without creating a database file. | Must | US-006; Report a missing database without creating a file |
| FR-015 | `mdsearch index status` against a database with no collections shall produce empty output. | Must | US-006; A fresh database with no collections produces empty output |
| FR-016 | `--database PATH` shall select the database used by `collection update` and `mdsearch index status`. | Must | US-006; Report a missing database without creating a file |
| FR-017 | For a collection with a recorded semantic state, `mdsearch index status` shall additionally report the embedding model and the dimension its vectors were built at; collections without a semantic state row report nothing extra. | Must | US-015; Status reports the recorded embedding model and dimension |
| FR-017 | `collection update --all` shall rebuild the lexical index for every collection. | Must | US-006; Update --all rebuilds the index for every collection |

## Postconditions And Invariants

- After a successful update, the lexical index reflects the reconciled file set:
  each stored file's body paragraphs and recognized frontmatter fields are
  indexed, and removed files contribute no passages.
- The index build and the file reconciliation commit or fail together; there is
  no observable intermediate state.
- A collection's index state is `built` only after a successful update.
- Frontmatter leniency is preserved: absent or malformed frontmatter never fails
  an update.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| A storage error occurs while building the index | The whole update fails and nothing is committed | The operation fails; the previous index state is unchanged |
| A file has malformed or unparseable frontmatter | The file is indexed body-only and the case is reported | The update succeeds and reports the malformed frontmatter |
| A file has no frontmatter | The file is indexed from its body paragraphs only | The update succeeds |
| A file is empty | The file contributes no passages | The update succeeds and the collection still shows `built` |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |
| The database exists with no collections | Nothing to report | Empty output |
| A collection has never been updated | Its index state remains `not built` | State `not built` in the status report |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for both commands.
- Indexing shall be driven by the explicit update command; there is no file
  watching.
- The lexical index shall be suitable for future BM25-ranked passage retrieval
  without redesign (the search command itself is a later EPIC-003 slice).

## Traceability

- Source story: `US-006` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`