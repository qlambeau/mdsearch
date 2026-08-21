---
id: REQ-014
title: "Unify literal free-text query semantics across lexical and hybrid search requirements"
type: feature-requirements
status: draft
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-014
related:
  - US-007
  - US-011
  - REQ-007
  - REQ-011
  - ADR-004
---

# Requirements

## Purpose And Actors

### Purpose

Unify the query interpretation of `mdsearch search` and `mdsearch hybrid` so
the identical query string produces the same matching passages on both
commands: the query is literal free text (whitespace-separated terms AND-joined
and quoted), FTS5 operator characters are inert, and a query that previously
failed as malformed FTS5 syntax now succeeds with literal semantics. The
`InvalidQuery` error contract remains as deterministic, defense-in-depth
classification that is unreachable for normal input, and the ADR-004 golden
evaluation is re-baselined after the change. The feature completes EPIC-008.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- The ADR-004 evaluation harness (`cargo xtask eval`) and its golden fixtures.

## Preconditions

- The user invokes `mdsearch search QUERY` or `mdsearch hybrid QUERY` with the
  respective command's existing switches (`--collection`, `--limit`, `--json`,
  `--no-rerank`, `--database`).
- The database exists; the search command's collection/index preconditions and
  the hybrid command's semantic-index and re-ranker preconditions are unchanged
  from `REQ-007` and `REQ-011`.
- The shared free-text-to-FTS5 mapping exists in the domain (already owned and
  unit-tested; reused, not invented).

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Lexical search | `QUERY`, optional `--limit N`, optional `--collection NAME`, optional `--database PATH` | Ranked passage blocks plus total-count summary when matches exist; empty output when none | Query is non-empty literal free text; `--limit` within 1..=100; unchanged otherwise |
| Hybrid search | `QUERY`, optional `--limit N`, optional `--collection NAME`, optional `--json`, optional `--no-rerank`, optional `--database PATH` | Ranked passage blocks per `REQ-011`; empty output when none | Query is non-empty literal free text; `--limit` within 1..=100; unchanged otherwise |
| Golden evaluation | `cargo xtask eval` | Recall@5, MRR@5, NDCG@5 against the ADR-004 targets | Targets remain met; baseline updated if scores shift |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `search` and `hybrid` shall interpret the query string with the identical query-to-FTS5 mapping: whitespace-separated terms are AND-joined, each term wrapped in double quotes, and embedded quotes doubled. | Must | US-014; The same query string returns the same passages on both commands; FTS5 operator characters match literally on both commands |
| FR-002 | FTS5 operator characters typed in a query (`AND`, `OR`, `NOT`, `*`, `^`, `~`, parentheses, hyphens, quotes) shall be treated as literal text on both commands and shall never alter match semantics. | Must | US-014; FTS5 operator characters match literally on both commands |
| FR-003 | An empty or whitespace-only query shall fail with a clear error on both commands. | Must | US-014; An empty query fails on both commands; A whitespace-only query fails on both commands |
| FR-004 | For the identical query string against identical collection state, `search` and `hybrid` shall return the same matching passages (a passage is returned, or not returned, identically); per-command ranking, scoring, and output shape remain each command's own contract. | Must | US-014; The same query string returns the same passages on both commands |
| FR-005 | A query that previously failed as malformed FTS5 syntax (e.g., `a AND`) shall succeed with literal-term semantics and shall not fail as a query error from normal input. | Must | US-014; 007 regression scenario Treat FTS5 operator characters as literal text |
| FR-006 | The `InvalidQuery` error contract shall remain as defense-in-depth, but classification shall be deterministic — independent of engine error-message text — and unreachable for normal user input. | Must | US-014 (AC); OBS-003 |
| FR-007 | After the change, `cargo xtask eval` shall re-run against the ADR-004 targets; if scores shift, the recorded baseline shall be updated, and Recall@5 >= 0.85, MRR@5 >= 0.70, and NDCG@5 >= 0.75 shall remain met. | Must | US-014; Re-run the golden evaluation after the change |
| FR-008 | All other `search` and `hybrid` contracts — `--limit` range, `--collection` handling, `--json`, `--no-rerank`, `--database`, output shapes, and ranking semantics — shall remain unchanged by this feature. | Must | US-014 (scope boundaries) |

## Postconditions And Invariants

- Both commands remain read-only: they never modify indexes, stored files,
  collections, or the database.
- The identical query string yields the identical matching passage set on both
  commands for the same collection state.
- No FTS5 operator syntax can be triggered from normal user input on either
  command; `InvalidQuery` is unreachable for normal input.
- The query-to-FTS5 mapping is deterministic: the same query string always
  produces the same FTS5 expression.
- The golden evaluation baseline is current with the implemented behavior.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Empty or whitespace-only query | Fail without searching | Clear error on both commands |
| Query containing operator characters (`a AND`, `rust OR ownership`, `borrowing*`) | Match literally | Only passages containing all literal terms |
| Query that previously failed as malformed (`a AND`) | Succeed | Literal-term results |
| Hybrid command's stale semantic index or uncached re-ranker | Unchanged `REQ-011` behavior | Per `REQ-011` |
| No passage matches | Return nothing | Empty output |
| `cargo xtask eval` scores shift | Baseline updated | ADR-004 targets still met |

## Quality Requirements

- Consistency: both commands consume the same domain-owned mapping; no
  per-command query interpretation may diverge from it.
- Determinism: neutralization is a pure function — the same query string always
  maps to the same FTS5 expression, verified by the domain unit tests.
- No additional I/O or latency beyond the existing commands: query
  interpretation is an in-memory transformation.
- The re-baselined golden set remains the quality gate per ADR-004.

## Dependencies And Deferred Decisions

- Reuses the existing domain neutralizer `free_text_to_fts5`
  (`crates/domain/src/fusion.rs`) with its unit and property tests; no new
  neutralization behavior is introduced.
- The concrete deterministic `InvalidQuery` classification mechanism
  (pre-validation vs. rusqlite error variant vs. SQLite extended result code)
  is deferred to design (story OQ-001).
- Golden re-baselining follows the ADR-004 strategy and harness.
- `REQ-007` and the `007-lexical-search` scenarios are revised as part of this
  feature so the approved behavior change and the existing contract stay in
  lockstep (R-SDD-05).

## Traceability

- Source story: `US-014` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-008, DEC-013)