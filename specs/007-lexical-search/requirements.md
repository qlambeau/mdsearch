---
id: REQ-007
title: "Search the lexical index for ranked passages requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-007
related:
  - US-006
  - REQ-006
  - DES-006
  - ADR-001
  - DB-001
  - TABLE-004
  - TABLE-005
---

# Requirements

## Purpose And Actors

### Purpose

Provide a dedicated lexical search command that returns passages ranked by BM25
relevance from the built FTS5 index, so a developer-curator can find the most
relevant passages across collections and a coding-agent harness can feed top-K
passages into an LLM prompt.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch search QUERY` with optional `--collection NAME`,
  `--limit N`, and `--database PATH`.
- At least one collection in the database has a built lexical index, or
  `--collection` names one.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Search all collections | `QUERY`, optional `--limit N`, optional `--database PATH` | Ranked passage blocks plus a total-count summary when matches exist; empty output when none | Query is non-empty and valid FTS5 syntax; `--limit` within 1..=100 |
| Search one collection | `QUERY`, `--collection NAME`, optional `--limit N`, optional `--database PATH` | Ranked passage blocks restricted to the named collection | `NAME` matched case-insensitively against an existing, built collection; query and limit valid |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `mdsearch search QUERY` shall search the built lexical index of every collection. | Must | US-007; Search all collections returns ranked passages |
| FR-002 | `--collection NAME` shall restrict the search to one collection, matched case-insensitively. | Must | US-007; Restrict a search to one collection |
| FR-003 | `--limit N` shall cap the number of displayed results, defaulting to 10 and accepting 1 through 100 inclusive; out-of-range values shall fail. | Must | US-007; Cap results with --limit and report the total; Reject an out-of-range --limit |
| FR-004 | Results shall be ranked by the FTS5 BM25 score, highest first. | Must | US-007; Search all collections returns ranked passages |
| FR-005 | Equal-score results shall be ordered by collection name, then file path, then passage position. | Must | US-007; Search all collections returns ranked passages |
| FR-006 | The query shall accept full FTS5 match syntax; an empty or whitespace-only query shall fail, and a malformed query shall fail with a clear error rather than a crash. | Must | US-007; Match an exact phrase; Fail clearly on a malformed query; Fail on an empty query |
| FR-007 | When searching all collections, collections with an unbuilt index shall be skipped silently. | Must | US-007; Skip unbuilt collections when searching all |
| FR-008 | `--collection` naming an unknown collection shall fail and report the collection was not found. | Must | US-007; Report a missing collection for --collection |
| FR-009 | `--collection` naming a collection with an unbuilt index shall fail and report that the index is not built. | Must | US-007; Report an unbuilt index for --collection |
| FR-010 | Each result block shall report the rank, the file path, the passage kind, the passage text, and the BM25 score. | Must | US-007; Search all collections returns ranked passages |
| FR-011 | When at least one result exists, the output shall end with a summary line reporting the total match count. | Must | US-007; Cap results with --limit and report the total |
| FR-012 | When no passage matches the query, the output shall be empty. | Must | US-007; Produce empty output when nothing matches |
| FR-013 | `--database PATH` shall select the database; a missing database shall fail without creating a file. | Must | US-007; Report a missing database without creating a file |

## Postconditions And Invariants

- A successful search mutates nothing in the database.
- Results come only from collections with a built lexical index.
- Result ordering is deterministic for identical inputs and index state.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Empty or whitespace-only query | Fail without searching | A clear error |
| Malformed query (e.g., `a AND`) | Fail without searching | A clear error naming the query problem |
| `--limit` outside 1..=100 | Fail | A clear error |
| `--collection` does not match a collection | Fail | Output communicates the collection was not found |
| `--collection` matches a collection with an unbuilt index | Fail | Output communicates the index is not built |
| Unbuilt collections exist when searching all | Skip them | Only built collections' matches are returned |
| No passage matches the query | Return nothing | Empty output |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |
| The database has no collections or no built index | Return nothing | Empty output |

## Quality Requirements

- The operation shall work offline and shall not require a network service.
- Human-readable output shall be the default output form for this command.
- Search shall be read-only and shall not alter the index or the stored files.
- Latency shall be suitable for harness context-filling calls (soft target,
  unconstrained by an explicit bound in this slice).

## Traceability

- Source story: `US-007` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`