---
id: TASK-011
title: "Hybrid search with lexical-semantic fusion and cross-encoder re-ranking implementation tasks"
type: implementation-tasks
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: US-011
related:
  - REQ-011
  - DES-011
  - US-010
  - REQ-010
  - DES-010
  - ADR-004
  - ADR-006
  - ADR-007
  - DB-001
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Tasks

## Implementation Approach

Implement the smallest complete path for `mdsearch hybrid`: domain value types,
RRF fusion, and the free-text-to-FTS5 mapper; a `Reranker` port and
`HybridSearchStore` port with the `HybridSearch` use case; a `FastembedReranker`
adapter in the existing `embed-fastembed` crate using fastembed's `TextRerank`
(no new dependency, ADR-007); a read-only `SqliteHybridSearchStore` over the
existing schema-v5 tables; the `embed --reranker` provisioning extension with a
global `reranker_model` setting; and the `hybrid` CLI command with human and
`--json` output. Keep `REQ-011` and the state flow in `DES-011` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification. No new dependency or workspace member is in scope: `fastembed`
6.0.0 already provides `TextRerank` (ADR-007). The `embed --reranker` extension
revises `REQ-010` and `DES-010`; those revisions are part of TASK-011-7 and must
be approved before the embed code changes.

This slice does not implement entity graphs, related-concept links, JSON changes
to `mdsearch search`, or empirical tuning of the RRF `k` / oversample factor /
re-ranker model (deferred to the ADR-004 evaluation framework).

## Ordered Tasks

- [x] **TASK-011-1:** Add domain types and pure functions: `RerankerModel`
      (validated non-empty model name), `reciprocal_rank_fusion` that fuses two
      ranked candidate lists keyed on the logical passage identity
      `(file_id, kind, position)` with the RRF constant `k = 60`, and a pure
      free-text-to-FTS5 mapper that quotes each whitespace-separated term and
      joins them with `AND`, neutralizing FTS5 operator characters.
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; `RerankerModel` rejects
    empty names; fusion is deterministic, sums `1 / (k + rank)` per list, and
    tie-breaks by `(file_id, kind, position)`; the mapper quotes terms, joins
    with `AND`, and treats `AND`/`OR`/quotes/`prefix*` as literal text.

- [x] **TASK-011-2:** Add the `Reranker` port with the `RerankError` type
      (`UnsupportedModel`, `ModelNotCached`, `DownloadFailed`, `Storage`), and
      an in-memory fake for tests.
  - Depends on: TASK-011-1
  - Verification: `cargo test -p kv-application` passes; tests cover the error
    mapping and the fake honours the port contract.

- [x] **TASK-011-3:** Add the `HybridSearchStore` port (`candidates`) that
      resolves the search scope, checks per-collection semantic staleness, and
      returns oversampled lexical and semantic candidate lists (`HybridCandidate`
      carrying file path, kind, text, and per-leg score) with
      `HybridSearchStoreError` (`CollectionNotFound`, `IndexNotBuilt`,
      `StaleSemanticIndex`, `Storage`), plus an in-memory fake for tests.
  - Depends on: TASK-011-1
  - Verification: `cargo test -p kv-application` passes; contract tests cover
    every port method and its error path.

- [x] **TASK-011-4:** Add the `HybridSearch` use case with the hybrid result set
      and `HybridError`. It validates the free-text query, resolves the scope,
      checks staleness before retrieval, retrieves oversampled candidates,
      fuses with RRF, re-ranks the whole fused pool when enabled and the model
      is available (falling back to RRF-only with a warning when uncached),
      cuts to `--limit`, and orders deterministically.
  - Depends on: TASK-011-2, TASK-011-3
  - Verification: `cargo test -p kv-application` passes; tests cover all and
    collection scopes, lexical-only fallback, staleness failure, re-rank on/off,
    uncached re-ranker warning, no-match, and empty-query rejection.

- [x] **TASK-011-5:** Implement `FastembedReranker` in the existing
      `crates/adapters/embed-fastembed` member using `fastembed::TextRerank`
      with the approved default model `bge-reranker-base`, a friendly-name
      mapping mirroring the embedding adapter, cache availability checks,
      download gating, and score output.
  - Depends on: TASK-011-2
  - Verification: `cargo deny check` passes (no new dependency); adapter tests
    confirm availability checks, download gating, unsupported-model mapping,
    and score shape (real inference is exercised offline, not in CI).
  - Note: `fastembed` 6.0.0 provides `TextRerank`; no new dependency or member
    is authorized (ADR-007).

- [x] **TASK-011-6:** Implement the read-only `SqliteHybridSearchStore` over the
      existing schema-v5 tables: the lexical leg reuses the FTS5
      `passages`/`passage_files` join with `AND`-joined quoted terms; the
      semantic leg embeds the query and runs `knn_match` on `embeddings` joined
      back to `passage_files`/`files`/`collections` with cosine distance
      converted to similarity; staleness compares the current stored file-set
      fingerprint against `semantic_index_state`.
  - Depends on: TASK-011-3
  - Verification: Store integration tests confirm lexical-leg retrieval,
    semantic-leg `knn_match` retrieval and distance-to-similarity conversion,
    stale-fingerprint detection, unknown collection, unbuilt lexical index, and
    deterministic ordering.

- [x] **TASK-011-7:** Extend `mdsearch embed` with `--reranker NAME` and
      `--download` to validate, cache-check, and provision the re-ranker model
      and record the global `reranker_model` setting in `settings`; revise
      `REQ-010` and `DES-010` to record the extension before the embed code
      changes.
  - Depends on: TASK-011-5
  - Verification: Embed CLI and store tests confirm model validation,
    availability/download gating, and the recorded global setting; the revised
    `REQ-010`/`DES-010` reference the `--reranker` flag and `reranker_model`
    setting.

- [x] **TASK-011-8:** Wire `mdsearch hybrid QUERY [--collection NAME]
      [--limit N] [--json] [--no-rerank] [--database PATH]` in the CLI; render
      human result blocks (rank, path, kind, text, ordering score) plus a
      shown-count summary, and `--json` with re-ranker/fused/BM25/cosine scores
      and provenance; map errors and the uncached-re-ranker warning.
  - Depends on: TASK-011-4, TASK-011-6
  - Verification: CLI acceptance tests mapped from `scenarios.feature` confirm
    rendering, `--collection` scope, `--limit` bounds, `--json` shape,
    `--no-rerank`, fallback-and-warn, empty query, empty output, staleness
    failure, and the missing-database boundary.

- [x] **TASK-011-9:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-011-8
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [ ] Unit checks: `RerankerModel` validation, `reciprocal_rank_fusion`
      determinism and change-detection, tie-breaking, and the free-text-to-FTS5
      mapper (quoting, `AND` joining, operator neutralization).
- [ ] Application checks: `HybridSearch` with fakes for every scope, staleness,
      lexical-only fallback, re-rank on/off, uncached re-ranker warning,
      no-match, and empty-query rejection.
- [ ] Integration checks: `SqliteHybridSearchStore` lexical-leg retrieval,
      semantic-leg `knn_match` and distance-to-similarity, stale-fingerprint
      detection, unknown collection, unbuilt lexical index, deterministic
      ordering.
- [ ] Adapter checks: `FastembedReranker` availability, download gating,
      unsupported-model mapping, and score shape.
- [ ] Embed extension checks: `embed --reranker` validation, gating, and global
      `reranker_model` setting; revised `REQ-010`/`DES-010`.
- [ ] CLI checks: rendering, scope, `--limit` bounds, `--json` shape,
      `--no-rerank`, fallback-and-warn, empty query, empty output, staleness,
      and missing database.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: offline default operation, no query-time network,
      read-only search, bounded pool (3 x limit), and no entity-graph behavior.
- [ ] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [ ] Regression check: confirm no hybrid/reranker behavior leaks into `search`,
      `get`, `update`, `add`, or the `embed` vector path beyond the documented
      `--reranker` extension.

## Rollout And Recovery

### Rollout

Ship `mdsearch hybrid` in the single compiled binary. No migration and no schema
change: the `reranker_model` setting lives in the existing `settings` table, and
hybrid search reads the existing schema-v5 tables. Re-ranker assets are not
bundled with the binary; they are loaded from the local cache or fetched only
when the user runs `mdsearch embed --reranker NAME --download`. Existing
databases need no migration.

### Recovery

A stale semantic index fails the whole `hybrid` command before retrieval with a
message naming the stale collection and directing the user to `mdsearch embed`;
re-running `embed` rebuilds the stale index and the next `hybrid` succeeds. An
uncached re-ranker degrades to RRF-only ordering with a warning rather than
failing; `--no-rerank` disables re-ranking without a warning. A failed
`embed --reranker --download` modifies no collection and no setting; re-running
it retries safely. A missing database fails without creating a file.

## Definition Of Done

- [ ] All tasks are complete.
- [ ] Automated unit, integration, and CLI checks pass.
- [ ] The executable scenarios pass.
- [ ] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [ ] Offline default and read-only constraints are verified.
- [ ] The `embed --reranker` extension and the revised `REQ-010`/`DES-010` are approved.
- [ ] No out-of-scope entity-graph, related-concept, search-change, or tuning behavior was added.
- [ ] Operational or documentation changes are complete.
