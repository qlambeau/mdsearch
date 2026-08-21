---
id: TASK-015
title: "Embed collections at the selected model's embedding dimension implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-015
related:
  - REQ-015
  - DES-015
  - US-010
  - US-011
  - REQ-010
  - REQ-011
  - REQ-006
  - ADR-004
  - ADR-010
  - DB-001
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Tasks

## Implementation Approach

Implement the approved changes from `DES-015`/`ADR-010` in dependency order:
first revise the affected specifications (`REQ-010`, `REQ-011`, `REQ-006`, and
the schema artifacts), then drive the store's dimension-aware table lifecycle
and read validation red-to-green, then the `index status` reporting, then the
CLI acceptance surface, then re-run the ADR-004 evaluation and the
constitution gates.

The production surface is confined to the SQLite adapter, the embed use-case
orchestration already owned by `REQ-010`, the `hybrid` read path, and the
`index status` renderer:

- `EMBEDDING_DIMENSION` is removed. The active dimension is recorded in the
  `settings` table (`embedding_dimension` key, `TABLE-007`); absent means 384.
- `rebuild` derives the dimension from the embedding batch it receives,
  records it in `semantic_index_state.dimension` (new column, additive
  migration with a schema-version bump), and recreates the `embeddings`
  virtual table at that dimension — transactionally with the settings write —
  when the recorded active dimension disagrees.
- The `hybrid` semantic leg validates each in-scope collection's recorded
  dimension against the active dimension before returning any results.
- `mdsearch index status` reports the recorded model and dimension for
  collections with a semantic state row.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces no new dependency and no CLI switch change. It does not
change hybrid ranking, fusion, lexical behavior, the default model, or the
supported model set.

## Ordered Tasks

- [x] **TASK-015-1:** Revise the affected specifications in lockstep
      (R-SDD-05): `REQ-010` (embed records model and dimension; model switch
      recreates the vector table and rebuilds every embedded collection at the
      new dimension), `REQ-011` (hybrid semantic-leg dimension validation),
      `REQ-006` (index status reports the semantic model and dimension), and
      the schema artifacts `TABLE-007` (`embedding_dimension` key),
      `TABLE-008` (`dimension` column, legacy NULL = 384), `TABLE-009`
      (dim = active dimension), and `DB-001` (schema-version bump and
      migration note). Add regression scenarios to the `010-semantic-index`
      and `011-hybrid-search` packets (1024-dimension embed; dimension
      mismatch on hybrid).
  - Depends on: None
  - Verification: the revised packets contain no 384-only claims; scenarios,
    requirements, and schema artifacts agree with `REQ-015`/`DES-015`.

- [x] **TASK-015-2 (RED):** Extend the semantic store integration tests
      (`crates/adapters/store-sqlite/tests/semantic_index_store.rs`): (a) a
      rebuild with 1024-length vectors succeeds and records the dimension; (b)
      a rebuild with a different batch dimension recreates the vector table
      and updates the active setting; (c) the rebuild guard names the expected
      and actual dimensions on mismatch; (d) a legacy state row without a
      recorded dimension is read as 384.
  - Depends on: TASK-015-1
  - Verification: the new tests fail red against the current constant-pinned
    implementation (observed failing output), except (d), which passes against
    today's schema shape.

- [x] **TASK-015-3 (RED):** Add hybrid-store integration tests
      (`crates/adapters/store-sqlite/tests/hybrid_search.rs`) asserting that a
      recorded dimension disagreeing with the active dimension fails the
      command before any results are returned.
  - Depends on: TASK-015-2
  - Verification: the mismatch test fails red against the current
    implementation, which performs no read-time dimension validation
    (observed failing output).

- [x] **TASK-015-4 (RED):** Add a status test (store or CLI acceptance level)
      asserting `mdsearch index status` reports the recorded semantic model
      and dimension for an embedded collection, and nothing extra for a
      collection without a semantic state row.
  - Depends on: TASK-015-3
  - Verification: the test fails red because the status surface is
    lexical-only today (observed failing output).

- [x] **TASK-015-5 (GREEN):** Implement the dimension-aware store: remove
      `EMBEDDING_DIMENSION`; record and read `embedding_dimension` in
      `settings` (absent = 384); create the `embeddings` virtual table on
      demand at the batch dimension and recreate it — transactionally with the
      settings write — when the active dimension disagrees; write `dimension`
      into `semantic_index_state` at rebuild (additive migration, legacy NULL
      read as 384); validate vector lengths against the active dimension with
      an error naming expected and actual dimensions; validate recorded
      dimensions on the hybrid semantic leg before results.
  - Depends on: TASK-015-4
  - Verification: TASK-015-2 and TASK-015-3 tests pass green; the full
      `kv-store-sqlite` suite still passes.

- [x] **TASK-015-6 (GREEN):** Report the recorded semantic model and dimension
      in `mdsearch index status` (use case and renderer), with no output
      change for collections without a semantic state row.
  - Depends on: TASK-015-5
  - Verification: TASK-015-4 passes green; existing `index status` tests
      remain green.

- [x] **TASK-015-7:** Add CLI acceptance tests mapped from
      `scenarios.feature` for the offline-reachable paths (status reporting,
      dimension mismatch error, legacy behavior, rebuild under a different
      model at the application layer with fakes) and from the revised
      `010-semantic-index`/`011-hybrid-search` packets.
  - Depends on: TASK-015-6
  - Verification: every offline-reachable scenario in the 015 packet and the
      revised 010/011 packets passes as an executable acceptance test.

- [x] **TASK-015-8:** Re-run `cargo xtask eval` (the golden set uses the
      default model; no baseline shift is expected) and execute the
      constitution gates and Definition of Done: `cargo xtask ci` with
      observed output; confirm the README model documentation is consistent
      with the supported set; confirm `US-015`, `REQ-015`, `DES-015`,
      `ADR-010`, and the revised packets agree.
  - Depends on: TASK-015-7
  - Verification: observed command output from every gate, per R-AGT-07; eval
      thresholds (Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75) still hold;
      no new warnings, suppressions, or unapproved artifacts.

## Test And Verification Plan

- [x] Unit checks: dimension derivation and legacy-384 inference helpers.
- [x] Integration checks: 1024-dimension rebuild, table recreation on
      dimension change, guard error naming, legacy NULL handling, hybrid
      read-time mismatch, status reporting (TASK-015-2, TASK-015-3,
      TASK-015-4).
- [x] CLI checks: `index status` model/dimension lines and the offline-reachable
      015 scenarios (TASK-015-7).
- [x] Gherkin scenarios: `scenarios.feature` (015) and the revised
      `010-semantic-index` and `011-hybrid-search` packets.
- [x] Non-functional checks: no new dependencies or workspace members, no CLI
      switch changes, hybrid ranking/fusion untouched, offline behavior
      unchanged.
- [x] Evaluation checks: `cargo xtask eval` with recorded output and verified
      ADR-004 thresholds (TASK-015-8).
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates
      with observed output (TASK-015-8).

## Rollout And Recovery

### Rollout

Ship the change in the single compiled binary. The migration is additive: a new
nullable `dimension` column on `semantic_index_state` (legacy rows read as 384)
and a new `settings` key; existing 384-dimension databases need no rebuild and
no data movement. `index status` gains a line item for embedded collections;
hybrid output shape is unchanged except for the new dimension-mismatch error.

### Recovery

A failed rebuild rolls back atomically per collection (existing REQ-010
contract); the previous vectors and recorded state stay intact. The only
destructive operation is the vector-table recreation during a model switch,
which commits with the `embedding_dimension` settings write and is immediately
followed by the rebuild of every embedded collection (REQ-010 FR-007); a crash
mid-switch leaves the previous table and state intact, and re-running `embed`
completes the switch. Reads never modify state and never return partial
results on a dimension disagreement.

## Definition Of Done

- [x] All tasks are complete.
- [x] Tests were written first and observed to fail (TASK-015-2, TASK-015-3,
      TASK-015-4).
- [x] Every behavior in `REQ-015` is covered and traceable to a test
      (R-SDD-02).
- [x] The executable scenarios for 015 and the revised 010/011 packets pass.
- [x] The Rust constitution's tooling gates pass with observed output
      (R-TOOL-04, R-AGT-07).
- [x] `cargo xtask eval` thresholds hold with recorded output.
- [x] `US-015`, `REQ-015`, `DES-015`, `ADR-010`, `REQ-010`, `REQ-011`,
      `REQ-006`, and the schema artifacts are consistent (R-SDD-05).
- [x] No new dependencies, workspace members, layers, unsafe code, or lint
      suppressions were introduced.
- [x] No out-of-scope behavior (ranking, fusion, default model, CLI switches)
      was added or changed.