---
id: TASK-014
title: "Unify literal free-text query semantics implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-014
related:
  - REQ-014
  - DES-014
  - US-007
  - REQ-007
  - ADR-004
  - ADR-009
---

# Tasks

## Implementation Approach

Implement the two approved changes from `DES-014`/`ADR-009` in dependency
order: first revise the stale `US-007` claims, then harden the domain mapper
with a structural property test, then drive the lexical use case and the store
error mapping red-to-green, then cover the CLI acceptance surface, then
re-baseline the ADR-004 evaluation.

The implementation touches exactly two production sites:
`SearchLexical::execute` (`crates/application/src/lexical_search.rs`) applies
the existing domain mapper `free_text_to_fts5` before delegating to the store,
and `search_query_failure`
(`crates/adapters/store-sqlite/src/lib.rs:1614-1623`) stops matching engine
message text and maps query-path execution failures to `SearchStoreError::Storage`.
`InvalidQuery` variants remain in the error enums as unreachable
defense-in-depth (FR-006).

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces no dependency, schema change, migration, or workspace
member, and changes no CLI switch or output shape. It does not touch ranking,
fusion, `--related`, `context`, `get`, or JSON rendering.

## Ordered Tasks

- [x] **TASK-014-1:** Revise `specs/007-lexical-search/user-story.md` so it no
      longer documents raw FTS5 match syntax: replace the business rule that
      "the query uses full FTS5 match syntax (`AND`/`OR`/`NOT`, `prefix*`)",
      the malformed-query example (EX-009), and the acceptance criteria with
      literal free-text semantics (terms quoted and AND-joined, operator
      characters inert, previously-malformed queries succeed).
  - Depends on: None
  - Verification: the 007 story contains no "full FTS5 syntax" or
    "malformed query" claims; its rules, examples, and criteria are consistent
    with the revised `007-lexical-search/scenarios.feature`, `REQ-007`, and
    `REQ-014` (R-SDD-05).

- [x] **TASK-014-2:** Add a domain property test for `free_text_to_fts5`
      (`crates/domain/src/fusion.rs`) asserting the canonical structure for
      arbitrary non-empty free text: every whitespace-separated term appears
      as a quoted phrase, embedded quotes are doubled, terms are joined with
      ` AND `, and no bare operator token survives.
  - Depends on: TASK-014-1
  - Verification: `cargo test -p kv-domain` passes; the property covers
    operator characters, quotes, and repeated whitespace (DES-014
    validity-by-construction claim).

- [x] **TASK-014-3 (RED):** Extend the application tests
      (`crates/application/tests/lexical_search.rs`) so the in-memory fake
      records the query string it receives, and add tests asserting that an
      operator-character query (e.g., `a AND b`) reaches the store as the
      neutralized expression (`"a" AND "AND" AND "b"`) and that a
      whitespace-only query is still rejected with `EmptyQuery`.
  - Depends on: TASK-014-2
  - Verification: tests fail red against the current implementation, which
    forwards the raw query (observed failing output).

- [x] **TASK-014-4 (RED):** Replace the store test
      `reports_a_malformed_query`
      (`crates/adapters/store-sqlite/tests/lexical_search.rs:259-275`) with
      regression tests: (a) a neutralized operator-character expression
      executes with literal semantics and returns only passages containing all
      literal terms; (b) a raw malformed expression such as `a AND` maps to
      `SearchStoreError::Storage` and never to `InvalidQuery`.
  - Depends on: TASK-014-3
  - Verification: test (b) fails red against the current message-text
    heuristic (observed failing output); test (a) passes against a real FTS5
    index.

- [x] **TASK-014-5 (GREEN):** Implement the approved changes: apply
      `free_text_to_fts5` in `SearchLexical::execute` after the existing
      trim-empty guard; revise `search_query_failure` to map query-path
      execution failures to `SearchStoreError::Storage` without matching
      engine message text; update the `LexicalSearchStore::search` port doc
      (`crates/application/src/ports/lexical_search_store.rs`) to state that
      the query argument is a neutralized expression and `InvalidQuery` is
      defense-in-depth.
  - Depends on: TASK-014-4
  - Verification: TASK-014-3 and TASK-014-4 tests pass green; existing
    application and store suites still pass (`cargo test -p kv-application`
    and `cargo test -p kv-store-sqlite` or the workspace equivalent).

- [x] **TASK-014-6:** Add CLI acceptance tests mapped from
      `scenarios.feature`: the same query string returns the same passages on
      `search` and `hybrid`; operator-character queries match literally on both
      commands (outline rows); empty and whitespace-only queries fail on both.
  - Depends on: TASK-014-5
  - Verification: every offline-reachable scenario in
      `specs/014-unify-query-semantics/scenarios.feature` and the revised
      `specs/007-lexical-search/scenarios.feature` passes as an executable
      acceptance test.

- [x] **TASK-014-7:** Re-baseline the ADR-004 evaluation: run `cargo xtask eval`
      against the current implementation and record the scores before
      changing behavior, then re-run after TASK-014-6 and compare.
  - Depends on: TASK-014-6
  - Verification: observed before/after eval output is recorded; if scores
    shift, the baseline is updated and the thresholds (Recall@5 >= 0.85,
    MRR@5 >= 0.70, NDCG@5 >= 0.75) still hold; a threshold breach stops the
    work for investigation before the baseline is updated.

- [x] **TASK-014-8:** Execute the constitution gates and Definition of Done:
      `cargo xtask ci` with observed output (fmt, clippy `-D warnings`, test,
      doc, deny, `llvm-cov` thresholds); confirm the README contains no raw
      FTS5 query-syntax claims; confirm `US-007`, the 007 scenarios/`REQ-007`,
      `REQ-014`, `DES-014`, and `ADR-009` are mutually consistent.
  - Depends on: TASK-014-7
  - Verification: observed command output from every gate, per R-AGT-07; no
    new warnings, suppressions, or unapproved artifacts.

## Test And Verification Plan

- [x] Unit checks: domain property test on `free_text_to_fts5` structure
      (TASK-014-2).
- [x] Application checks: `SearchLexical` forwards the neutralized expression
      and rejects whitespace-only queries with a fake store (TASK-014-3).
- [x] Integration checks: literal semantics for operator-character expressions
      against a real FTS5 index; deterministic `Storage` mapping for execution
      failures (TASK-014-4).
- [x] CLI checks: identical passage sets across commands, operator literals,
      empty/whitespace rejection (TASK-014-6).
- [x] Gherkin scenarios: `scenarios.feature` (014) and the revised
      `007-lexical-search/scenarios.feature`.
- [x] Non-functional checks: read-only behavior, offline execution, no new
      workspace members or dependencies, no CLI or output-shape changes,
      hybrid behavior unchanged.
- [x] Evaluation checks: `cargo xtask eval` before/after with recorded scores
      and verified ADR-004 thresholds (TASK-014-7).
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates
      with observed output (TASK-014-8).

## Rollout And Recovery

### Rollout

Ship the change in the single compiled binary. No migration and no schema
change are required; existing databases are unaffected. The behavior change is
user-visible only for queries containing FTS5 operator characters, which now
match literally (the intended, documented behavior in `REQ-014`/`REQ-007`).
The ADR-004 golden scores are recorded before and after the change and
re-baselined only if the thresholds still hold.

### Recovery

A failed search writes nothing and mutates no state; re-running with corrected
arguments retries safely. A missing database fails without creating a file. If
the post-change evaluation breaches a threshold, the work stops and the shift
is investigated before any baseline is updated; the pre-change eval output
recorded in TASK-014-7 provides the rollback comparison point. No state or
index rollback is ever needed because the commands are read-only.

## Definition Of Done

- [x] All tasks are complete.
- [x] Tests were written first and observed to fail (TASK-014-3, TASK-014-4).
- [x] Every behavior in `REQ-014` is covered and traceable to a test
      (R-SDD-02).
- [x] The executable scenarios for 014 and the revised 007 packets pass.
- [x] The Rust constitution's tooling gates pass with observed output
      (R-TOOL-04, R-AGT-07).
- [x] `cargo xtask eval` thresholds hold; the baseline is current.
- [x] `US-007`, 007 scenarios/`REQ-007`, `REQ-014`, `DES-014`, and `ADR-009`
      are consistent (R-SDD-05).
- [x] No new dependencies, workspace members, layers, unsafe code, or lint
      suppressions were introduced.
- [x] No out-of-scope behavior (ranking, fusion, `--related`, `context`,
      `get`, JSON shapes) was added or changed.