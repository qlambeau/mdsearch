---
id: REQ-008
title: "Show passage positions and machine-readable JSON for search requirements"
type: feature-requirements
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-008
related:
  - US-007
  - US-006
  - REQ-007
  - DES-007
  - ADR-001
  - DB-001
  - TABLE-005
---

# Requirements

## Purpose And Actors

### Purpose

Extend the lexical search command so each result reports its position in the
source file in the default human output, and add a `--json` switch that emits a
richer, machine-readable result for coding-agent harnesses.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  path override.

## Preconditions

- The user invokes `mdsearch search QUERY` with optional `--json`,
  `--collection NAME`, `--limit N`, and `--database PATH`.
- The collection(s) searched have a built lexical index, and the index records
  each passage's byte offset in its file.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Human search | `QUERY`, optional `--collection`, `--limit`, `--database` | Ranked blocks with `PATH:START-END (KIND, score SCORE)` headers plus a total summary; empty output when nothing matches | Query non-empty and valid FTS5 syntax; `--limit` within 1..=100 |
| JSON search | `QUERY`, `--json`, optional `--collection`, `--limit`, `--database` | One JSON object with query, scope, limit, total, and `results[]`; valid JSON even with zero matches | Same as human, plus a well-formed JSON document |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | Default human search output shall show each result's line range in its file as `PATH:START-END` in the block header. | Must | US-008; Show the passage line range in the human output |
| FR-002 | `--json` shall emit exactly one JSON object with the query, the collection scope, the limit, the total match count, and a `results` array. | Must | US-008; Emit a structured JSON object for a search |
| FR-003 | Each JSON result shall report the collection, file path, passage kind, passage text, score, and position (the line range and the byte range in the file). | Must | US-008; Emit a structured JSON object for a search |
| FR-004 | With `--json` and zero matches, the command shall emit a valid JSON object with an empty `results` array and total 0. | Must | US-008; Emit valid JSON with empty results when nothing matches |
| FR-005 | Without `--json`, zero matches shall produce empty output. | Must | US-008; Produce empty human output when nothing matches |
| FR-006 | All existing search behavior shall be preserved: BM25 ranking, deterministic tie-breaking, `--limit` bounds, `--collection`, full FTS5 query syntax, and clear errors for empty, malformed, unknown-collection, unbuilt-index, and missing-database cases. | Must | US-008; Fail clearly on a malformed query in JSON mode; Fail on an empty query in JSON mode |
| FR-007 | A result's position shall be computed from the passage's byte offset recorded in the index at build time. | Must | US-008; Show the passage line range in the human output |
| FR-008 | Errors shall be reported on stderr in both modes, and no JSON shall be emitted when the command fails. | Must | US-008; Fail clearly on a malformed query in JSON mode |

## Postconditions And Invariants

- A `--json` search produces a single, parseable JSON document on stdout.
- A result's line range and byte range accurately locate the passage in the
  stored file content.
- A successful search mutates nothing in the database.
- The operation does not require network access or an external service.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Zero matches, human mode | Print nothing | Empty output |
| Zero matches, `--json` mode | Print a valid JSON object | `results` array empty, total 0 |
| Malformed or empty query, either mode | Fail without output | Clear error on stderr; no JSON |
| `--limit` out of range, either mode | Fail | Clear error on stderr |
| Unknown or unbuilt `--collection`, either mode | Fail | Clear error on stderr |
| Missing database, either mode | Fail without creating a file | Clear error on stderr |

## Quality Requirements

- `--json` output shall always be well-formed JSON that a parser can consume.
- Positions shall be correct for the stored file content (1-based inclusive
  line range).
- The operation shall work offline and remain read-only.
- Human-readable output remains the default; JSON is opt-in.

## Traceability

- Source story: `US-008` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`