---
id: TASK-010
title: "Build the semantic index with the embed command implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: US-010
related:
  - REQ-010
  - DES-010
  - ADR-001
  - ADR-003
  - ADR-004
  - ADR-005
  - ADR-006
  - DB-001
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Tasks

## Implementation Approach

Implement the smallest complete path for `mdsearch embed`: domain value types
and the file-set fingerprint, an `EmbeddingGenerator` port and
`SemanticIndexStore` port with the `EmbedCollections` use case, a `fastembed`
adapter crate that generates local vectors behind `--download`, a schema-v5
migration and `SqliteSemanticIndexStore` that stores vectors keyed to the
logical `(file_id, kind, position)` passage identity, and the `embed` CLI
command with `--collection`, `--model`, `--download`, and `--database`. Keep
`REQ-010` and the state flow in `DES-010` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification. The `fastembed` dependency and the new
`crates/adapters/embed-fastembed` workspace member are explicitly approved
(ADR-006); no other new dependency or member is in scope.

This slice does not implement hybrid search, semantic querying or ranking, JSON
output, related-concept links, or entity graphs.

## Ordered Tasks

- [x] **TASK-010-1:** Add domain types: `EmbeddingModel` (validated non-empty
      model name), `Embedding` (`Vec<f32>` vector), `SemanticPassage`
      (file, kind, position, text), and a pure `file_set_fingerprint` function
      that hashes a collection's stored file set (sorted paths and content
      hashes).
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; `EmbeddingModel` rejects
    empty names, `file_set_fingerprint` is deterministic and changes when the
    file set changes.

- [x] **TASK-010-2:** Add the `EmbeddingGenerator` port with the
      `EmbeddingError` type (`UnsupportedModel`, `ModelNotCached`,
      `DownloadFailed`, `Storage`), and an in-memory fake for tests.
  - Depends on: TASK-010-1
  - Verification: `cargo test -p kv-application` passes; tests cover the error
    mapping and the fake honours the port contract.

- [x] **TASK-010-3:** Add the `SemanticIndexStore` port
      (`global_model`, `set_global_model`, `status`, `embedded_collections`,
      `passages`, `file_set_fingerprint`, `rebuild`) with
      `SemanticIndexStoreError`, and an in-memory fake for tests.
  - Depends on: TASK-010-1
  - Verification: `cargo test -p kv-application` passes; contract tests cover
    every port method and its error path.

- [x] **TASK-010-4:** Add the `EmbedCollections` use case with `EmbedScope`,
      `EmbedReport` / `EmbedOutcome`, and `EmbedError`. It resolves the
      effective model (flag, then global, then default), validates availability
      before any collection work, resolves targets, applies staleness and
      model-switch rebuild-all logic, and reports per-collection outcomes.
  - Depends on: TASK-010-2, TASK-010-3
  - Verification: `cargo test -p kv-application` passes; tests cover all and
    collection scopes, model availability, staleness, model-switch rebuild-all,
    skip and fail paths, and report shaping.

- [x] **TASK-010-5:** Add the `fastembed` workspace dependency, create the
      `crates/adapters/embed-fastembed` member, and implement
      `FastembedGenerator` with the approved default model `all-MiniLM-L6-v2`,
      cache availability checks, `--download` gating, and embedding output.
  - Depends on: TASK-010-2
  - Verification: `cargo deny check` passes for the new dependency; adapter
    tests confirm availability checks, download gating, unsupported-model
    mapping, and embedding dimension (384).
  - Note: the approved human authorization for this dependency and member is
    recorded in `ADR-006`.

- [x] **TASK-010-6:** Implement the schema-v5 migration in `store-sqlite`
      (`settings`, `semantic_index_state`, `embeddings` vector table with
      `collection_id`/`file_id`/`kind`/`position` metadata columns) and the
      `SqliteSemanticIndexStore` implementing the port.
  - Depends on: TASK-010-3
  - Verification: Store integration tests confirm global-model get/set, status
    read/write, passage reads, fingerprint computation, atomic rebuild
    (success and rollback), rebuild-after-rebuild, and predicate delete on the
    vector table.

- [x] **TASK-010-7:** Wire `mdsearch embed [--collection NAME] [--model NAME]
      [--download] [--database PATH]` in the CLI; render the per-collection
      summary; map a report containing `Failed` outcomes to a non-zero exit
      while still printing the summary.
  - Depends on: TASK-010-4, TASK-010-5, TASK-010-6
  - Verification: CLI acceptance tests mapped from `scenarios.feature` confirm
    rendering, `--collection` scope, `--model` validation and switch,
    `--download`, skip and fail paths, partial-failure exit, and the
    missing-database boundary.

- [x] **TASK-010-8:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-010-7
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [ ] Unit checks: `EmbeddingModel` validation, `Embedding` construction,
      `SemanticPassage` accessors, and `file_set_fingerprint` determinism and
      change-detection.
- [ ] Application checks: `EmbedCollections` with fakes for every scope, model
      availability, staleness, model-switch rebuild-all, skip/fail paths, and
      report shaping.
- [ ] Integration checks: `SqliteSemanticIndexStore` global model, status,
      passages, fingerprint, atomic rebuild and rollback, rebuild-after-rebuild,
      and vector-table predicate delete.
- [ ] Generator checks: `FastembedGenerator` availability, download gating,
      unsupported-model mapping, and embedding dimensions.
- [ ] CLI checks: rendering, scope, model validation and switch, `--download`,
      skip/fail paths, partial-failure exit, and missing database.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: offline default operation, `--download` opt-in
      network use, atomic per-collection rebuild, and no hybrid-search behavior.
- [ ] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [ ] Regression check: confirm no hybrid search, semantic querying, JSON
      output, related-concept links, or entity-graph behavior is added.

## Rollout And Recovery

### Rollout

Ship `mdsearch embed` in the single compiled binary. The schema-v5 migration is
idempotent and applied in-place when the store is opened for embedding; existing
databases migrate forward without data loss. The `fastembed` model assets are
not bundled with the binary; they are loaded from the local cache or fetched
only when the user passes `--download`.

### Recovery

A failed per-collection rebuild rolls back atomically, leaving the previous
semantic index intact; the command reports the failure and continues with the
remaining collections, and its exit status reflects any failure. A failed
`--download` modifies no collection. Re-running `mdsearch embed` retries safely:
unchanged collections report already current and failed collections rebuild.
A missing database fails without creating a file. A collection whose lexical
index is not built is skipped (all-collections mode) or reported as not built
(explicit `--collection`).

## Definition Of Done

- [ ] All tasks are complete.
- [ ] Automated unit, integration, and CLI checks pass.
- [ ] The executable scenarios pass.
- [ ] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [ ] Offline default and `--download` opt-in constraints are verified.
- [ ] Relevant specifications are updated if implementation details require clarification.
- [ ] No out-of-scope hybrid-search, semantic-query, JSON, link, or entity-graph behavior was added.
- [ ] Operational or documentation changes are complete.
