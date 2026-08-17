---
id: US-008
title: "Show passage positions and machine-readable JSON for search"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-006
feature: 008-machine-output
related:
  - US-007
  - US-006
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want search results to show where each matching passage sits in its file and
to get a machine-readable JSON form,
so that I can locate passages precisely and feed structured results to a
harness.

## Context And Value

`US-007` returns ranked passages but gives no indication of where each passage
lies in its source file, and its output is human-only. This story adds the
passage's line range to the default human output and a `--json` switch that
emits a richer machine-readable result, so both the human can jump straight to
the passage and the harness can consume structured, positioned results. It is
the first slice of EPIC-006.

## Business Rules

- The existing `mdsearch search QUERY` command gains a `--json` switch and
  reports each result's position in its file.
- In the default human output, each result block header shows the passage's
  line range in the file: `RANK. PATH:START-END (KIND, score SCORE)`, followed
  by the passage text on the next line. The `N match(es)` summary line is
  unchanged.
- The line range is the 1-based inclusive first and last line of the passage in
  the file, computed from the passage's byte offset recorded at index time.
- With `--json`, the command emits exactly one JSON object to stdout with the
  query, the collection scope (all, or the named collection), the limit, the
  total match count, and a `results` array. Each result reports the collection,
  file path, passage kind, passage text, score, and position (the line range and
  the byte range in the file).
- With `--json` and zero matches, the command still emits a valid JSON object
  with an empty `results` array.
- Without `--json`, zero matches still produce empty output (unchanged).
- All existing search behavior is preserved: BM25 ranking, deterministic
  tie-breaking, `--limit` bounds, `--collection`, full FTS5 query syntax, clear
  errors for empty, malformed, unknown-collection, unbuilt-index, and
  missing-database cases.
- Errors are reported on stderr in both modes, exactly as before.
- Exact JSON field names, ordering, and the formatting of the score and positions
  are not part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A built collection has `notes.md` with the passage "borrowing rules" on lines 12-16 | I run `mdsearch search borrowing` | The result header shows `notes.md:12-16` and the passage text follows |
| EX-002 | The same collection | I run `mdsearch search borrowing --json` | One JSON object with the query, scope, limit, total, and a `results` array; each result has collection, path, kind, text, score, and position |
| EX-003 | No passage matches the query | I run `mdsearch search zzz --json` | A valid JSON object is emitted with an empty `results` array and total 0 |
| EX-004 | No passage matches the query | I run `mdsearch search zzz` | Empty output (unchanged) |
| EX-005 | A query is malformed | I run `mdsearch search "a AND" --json` | The command fails on stderr with a clear error; no JSON is emitted |
| EX-006 | A query is empty | I run `mdsearch search "" --json` | The command fails on stderr with a clear error |
| EX-007 | `--limit 5` is supplied | I run `mdsearch search borrowing --json --limit 5` | The JSON `results` array has at most 5 entries and `total` reports the full count |

## Acceptance Criteria

- Default human search output shows each result's line range in its file.
- `--json` emits a single structured JSON object with query, scope, limit,
  total, and a `results` array carrying collection, path, kind, text, score, and
  position.
- Zero matches produce empty human output but a valid JSON object with an empty
  results array under `--json`.
- Existing ranking, limits, collection restriction, and error behavior are
  preserved in both modes.

## Scope Boundaries

### In Scope

- Reporting passage line ranges in the default human search output.
- A `--json` switch on `mdsearch search` producing richer machine-readable
  output.
- Recording passage byte offsets at index time to support positions.

### Out Of Scope

- A command to retrieve a complete file by name or ID (a later EPIC-006 slice).
- Related-concept links (EPIC-006, dependent on EPIC-005).
- JSON output for commands other than `search`.
- Semantic or hybrid retrieval output (EPIC-004).

## Dependencies

- `US-007` provides the search command whose output is extended.
- `US-006` provides the index build that records each passage's byte offset.

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