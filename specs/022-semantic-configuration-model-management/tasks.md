---
id: TASK-022
title: "Implement semantic configuration and model management"
type: implementation-tasks
status: implemented
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the US-022 task plan in chat on 2026-09-30"
parent: US-022
depends_on:
  - US-010
  - US-011
  - US-015
  - US-021
requires:
  - REQ-022
  - DES-022
  - ADR-016
  - ADR-017
  - DB-001
  - TABLE-002
blockers: []
related:
  - ADR-005
  - ADR-006
  - ADR-007
  - ADR-010
  - ADR-012
---

# Tasks

## Implementation Approach

Implement US-022 within the existing domain, application, SQLite adapter, model
adapter, and CLI layers. Write and observe all RED tests before production
changes. Extend current store boundaries; do not add crates, workspace members,
dependencies, or architectural layers. Stage model inference before writes and
preserve the approved atomicity scopes: one transaction per collection update,
and one transaction for a database-wide embedding-model change.

Keep the current command grammar during this slice. Add semantic configuration
and `model list`/`model set`; retain `embed` temporarily but require the
collection's semantic policy to be enabled. The final breaking command grammar,
unified retrieval, graph scope, and richer status remain separate work.

## Ordered Tasks

- [x] **TASK-022-1 (RED):** Add application/domain tests for semantic policy,
  model management, and update orchestration.
  - Depends on: Approved US-022 packet.
  - Outcome: Behavioral tests cover default-off and explicit configuration,
    enable/disable update behavior, disabled `embed` rejection, model listing,
    optional/reranker-only model selection, preflight failures, global rebuild
    targeting, and failure rollback using in-memory fakes.
  - Verification: Run focused domain and application targets and observe
    failures for the missing policy/model APIs and atomic contracts.
- [x] **TASK-022-2 (RED):** Add SQLite integration tests for schema v9,
  semantic-policy persistence, and atomic index transactions.
  - Depends on: TASK-022-1.
  - Outcome: Tests cover migrations from versions 0–8, legacy semantic-state
    backfill, create/configure/destroy policy behavior, file/lexical/graph/
    semantic rollback, disabled-vector cleanup, global setting/vector rollback,
    and shared vector-table dimension changes.
  - Verification: Run focused `kv-store-sqlite` targets and observe expected
    failures against the schema and persistence APIs.
- [x] **TASK-022-3 (RED):** Add CLI subprocess acceptance tests for semantic
  configuration and model commands.
  - Depends on: TASK-022-1.
  - Outcome: Tests cover create/configure semantic options, model list and
    availability, model-set validation and reranker-only mode, explicit
    downloads, model-change failure status, and legacy `embed` guidance.
  - Verification: Run focused `kv-app` integration targets and observe
    argument/operational failures and missing output behavior without network
    downloads.
- [x] **TASK-022-4 (GREEN):** Implement collection semantic policy and the
  schema-v9 migration.
  - Depends on: TASK-022-2.
  - Outcome: Persist the enabled/disabled policy, default new collections to
    disabled, backfill legacy collections from existing semantic-index state,
    and expose configuration through existing collection stores/use cases.
  - Verification: TASK-022-2 migration and policy tests pass from every
    supported prior schema version; create/configure operations do not index
  content.
- [x] **TASK-022-5 (GREEN):** Integrate semantic index staging into atomic
  source-aware collection updates.
  - Depends on: TASK-022-1, TASK-022-2, TASK-022-4.
  - Outcome: Prepare enabled semantic vectors before writing, then commit files,
    lexical index, graph, semantic vectors/state, or disabled-index cleanup
    within the existing collection transaction. Preserve update-all partial
    success. Reject legacy `embed` for disabled collections without changing
    policy.
  - Verification: Focused application and adapter tests pass; injected failures
    at inference and each index stage preserve the prior collection state.
- [x] **TASK-022-6 (GREEN):** Implement model catalog and atomic global model
  management.
  - Depends on: TASK-022-1, TASK-022-2, TASK-022-4.
  - Outcome: Report supported models and local availability through existing
    generator/re-ranker ports; support embedding, reranker-only, and combined
    selection; stage all enabled collection vectors before committing settings,
    dimension changes, vectors, and state in one transaction.
  - Verification: Tests cover unsupported and uncached models, download gating,
    single/mixed dimension transitions, all enabled collections despite
    narrow legacy embed scope, re-ranker-only changes, and complete rollback.
- [x] **TASK-022-7 (GREEN / TRACE):** Wire collection and model CLI commands and
  update the user documentation.
  - Depends on: TASK-022-3, TASK-022-5, TASK-022-6.
  - Outcome: Add semantic creation/configuration options and `model list`/
    `model set`; route existing `embed` model changes through the shared atomic
    operation; preserve help, diagnostics, global database behavior, and update
    the README with prerequisites and examples.
  - Verification: All new subprocess cases and existing app tests pass; help
    and README match tested switches and failure guidance.
- [x] **TASK-022-8 (VERIFY):** Trace scenarios and run repository quality and
  retrieval gates.
  - Depends on: TASK-022-7.
  - Outcome: Record passing test evidence, migration/rollback coverage, scenario
    traceability, and quality gate output in this file.
  - Verification: `cargo xtask validate-specs`, `cargo xtask ci`,
    `cargo xtask eval --verify-only`, and `cargo xtask eval` pass; `git diff
    --check` is clean.

## Test And Verification Plan

- [x] REQ-022 FR-001/FR-002: default-off, explicit create/configure policy, and
  configuration without ingestion.
- [x] REQ-022 FR-003/FR-004: semantic index removal when disabled and atomic
  file/lexical/graph/semantic updates, including failure rollback.
- [x] REQ-022 FR-005: update-all attempts every collection and reports partial
  failure with nonzero status.
- [x] REQ-022 FR-006: schema migrations from versions 0–8 preserve existing
  semantic enablement and do not enable collections without semantic state.
- [x] REQ-022 FR-007/FR-009/FR-010: model catalog, local availability,
  supported-name validation, explicit download behavior, and no state changes
  on preflight/download failures.
- [x] REQ-022 FR-008/FR-011/FR-012: model-set argument validation, global
  rebuild of enabled collections, dimension changes, whole-operation rollback,
  and reranker-only changes without vector rebuild.
- [x] REQ-022 FR-013: old `embed` command requires semantic enablement and does
  not enable policy implicitly.
- [x] All scenarios in `scenarios.feature` trace to passing unit, adapter, or
  subprocess tests; preserve existing semantic, hybrid, graph, lexical, and
  output contracts.
- [x] No test downloads model assets or uses network; use behavioral fakes and
  isolated temporary SQLite databases.
- [x] No new dependencies, crates, or workspace members; no weakened
  assertions, ignored tests, unsafe code, or unjustified lint suppressions.
- [x] Run `cargo xtask validate-specs`, `cargo xtask ci`,
  `cargo xtask eval --verify-only`, and `cargo xtask eval`; record actual output
  through `verify-feature`.

## Rollout And Recovery

### Rollout

Schema v9 adds the per-collection semantic policy. Existing semantic indexes
remain enabled based on their stored semantic state; other existing collections
remain disabled. New collections default to disabled. Configuration changes
take effect during the next successful update. The current `embed` command
remains available but requires semantic enablement; the later CLI slice will
replace the transitional command grammar.

### Recovery

A failed collection update rolls back its file changes and every index, so the
user can correct sources, permissions, model availability, or indexing errors
and retry. In update-all, completed collections remain committed while failed
collections retain their previous state. A failed global embedding-model
change rolls back the setting and every affected semantic index, leaving the
prior model usable. A successful model download may remain in the local cache
after a later database rollback; retrying can reuse it.

## Definition Of Done

- [x] All ordered tasks are complete with observed RED before GREEN evidence.
- [x] All approved US-022 scenarios pass and trace to tests.
- [x] Schema v9 migration and collection policy are tested from versions 0–8.
- [x] Per-collection and database-wide atomicity survive injected failure at
  inference, vector, state, dimension, and commit stages.
- [x] `cargo xtask validate-specs`, `cargo xtask ci`,
  `cargo xtask eval --verify-only`, and `cargo xtask eval` pass with observed
  output recorded.
- [x] Verification evidence includes date, commit/worktree state, commands, and
  outcomes.

### Observed Verification Evidence

Date: 2026-09-30. Base commit: `dca3122474898aafb49f15ad979fd524c2e86cc5`;
implementation changes are in the working tree and have not been committed.

- `cargo xtask validate-specs` — passed; 122 specification artifacts scanned.
- `cargo xtask ci` — passed on the elevated rerun after the sandbox could not
  acquire the cargo advisory database lock. Formatting, clippy with
  `-D warnings`, all-feature workspace tests, rustdoc with warnings denied,
  cargo-deny advisory/ban/license/source checks, and llvm-cov passed. Total line
  coverage was 86.80%, above the 85% threshold.
- `cargo xtask eval --verify-only` — passed; 32 documents, 32 queries, and 78
  judgments verified.
- `cargo xtask eval` — passed; overall Recall@5, MRR@5, and NDCG@5 were each
  1.0000, above all ADR-004 thresholds.
- Focused CLI, application, and adapter suites passed, including 7 semantic
  configuration subprocess tests, 24 embedding application tests, and 20
  semantic-index store tests.
- `git diff --check` — passed.
