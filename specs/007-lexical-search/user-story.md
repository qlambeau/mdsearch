---
id: US-007
title: "Search the lexical index for ranked passages"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-003
feature: 007-lexical-search
related:
  - US-006
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want to search the lexical index and get ranked passages,
so that I can find the most relevant passages across my collections for a query.

## Context And Value

`US-006` builds a per-passage FTS5 index during `collection update` but offers
no way to query it. This story adds the dedicated lexical search command that
returns passages ranked by BM25 relevance, completing EPIC-003. Both personas
benefit: the human reads ranked passages with their file paths, and the harness
can feed the top-K passages into an LLM prompt (JSON output and diff-style
positions remain a later EPIC-006 slice).

## Business Rules

- The command is `mdsearch search QUERY`, with optional `--collection NAME`,
  `--limit N`, and `--database PATH`.
- `--limit` defaults to 10 and accepts values from 1 through 100 inclusive.
- `--collection NAME` restricts the search to one collection, matched
  case-insensitively; without it, every collection is searched.
- The query uses full FTS5 match syntax: bare terms, `"quoted phrases"`,
  `prefix*`, and `AND` / `OR` / `NOT` operators.
- An empty or whitespace-only query fails with a clear error.
- A malformed query fails with a clear error naming the problem; it is never a
  crash.
- Results are ranked by the FTS5 BM25 score, highest first.
- Equal scores are ordered deterministically by collection name, then file
  path, then passage position.
- Only collections whose lexical index has been built are searched:
  - When searching all collections, collections with an unbuilt index are
    skipped silently.
  - When `--collection` targets a collection with an unbuilt index, the command
    fails and reports that the index is not built.
- Each result block reports the rank, the file path, the passage kind
  (`body`, `title`, `tags`, `aliases`, or `summary`), the passage text, and the
  BM25 score.
- When at least one result exists, the output ends with a summary line reporting
  the total number of matching passages (which may exceed the limit).
- When no collection matches the query, the output is empty.
- If the selected database does not exist, the command fails and reports the
  database does not exist without creating a file.
- Exact human-readable wording of errors and formatting of the score is not part
  of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | Collections `Notes` and `Archive` have built indexes containing passages about borrowing | I run `mdsearch search borrowing` | Ranked passage blocks from both collections, best score first |
| EX-002 | `Notes` has a built index | I run `mdsearch search borrowing --collection Notes` | Only `Notes` passages are returned |
| EX-003 | No collection named `Journal` exists | I run `mdsearch search borrowing --collection Journal` | The command fails and reports the collection was not found |
| EX-004 | `Notes` stores files but its index was never built | I run `mdsearch search borrowing --collection Notes` | The command fails and reports the index is not built |
| EX-005 | A collection's index was never built alongside built collections | I run `mdsearch search borrowing` | The unbuilt collection is skipped and matches from the built ones are returned |
| EX-006 | `--limit 5` is supplied | I run `mdsearch search borrowing --limit 5` | At most 5 passage blocks are shown and the summary reports the total count |
| EX-007 | `--limit 200` is supplied | I run `mdsearch search borrowing --limit 200` | The command fails and reports the limit is out of range |
| EX-008 | A query uses `"rust ownership"` | I run `mdsearch search "rust ownership"` | Only passages containing the exact phrase are returned |
| EX-009 | A query is malformed, for example `a AND` | I run `mdsearch search "a AND"` | The command fails with a clear error naming the query problem |
| EX-010 | A query is empty | I run `mdsearch search ""` | The command fails with a clear error |
| EX-011 | No passage matches the query | I run `mdsearch search zzzznotaword` | The output is empty |
| EX-012 | No database exists at the selected path | I run `mdsearch search borrowing --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- `mdsearch search QUERY` returns passage blocks ranked by BM25 score, each
  with rank, file path, passage kind, passage text, and score.
- `--limit N` (1 through 100, default 10) caps the number of blocks shown.
- `--collection NAME` restricts the search and matches case-insensitively.
- Unknown collections and unbuilt indexes targeted by `--collection` fail with
  clear errors.
- Searching all collections skips unbuilt indexes silently.
- Full FTS5 match syntax is accepted; empty and malformed queries fail with
  clear errors, never a crash.
- Equal-score results are ordered deterministically by collection name, file
  path, then passage position.
- A summary line reports the total match count when results exist; no matches
  produce empty output.
- A missing database fails without creating a file.

## Scope Boundaries

### In Scope

- Ranking and retrieving passages from the built lexical index.
- Full FTS5 query syntax with clear malformed-query errors.
- Result limiting, collection restriction, deterministic ordering, and the
  total-count summary.
- Human-readable ranked-passage output.

### Out Of Scope

- JSON output, diff-style positions, and provenance (EPIC-006).
- Retrieving a complete file by name or ID (EPIC-006).
- Related-concept links (EPIC-006).
- Semantic vectors, hybrid fusion, and entity graphs (EPIC-004 and EPIC-005).
- Changing how the lexical index is built (US-006 behavior).

## Dependencies

- `US-006` provides the built per-passage FTS5 index and the `passage_files`
  mapping that this command reads.
- FTS5 `bm25()` ranking is the approved scoring mechanism (ADR-001).

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