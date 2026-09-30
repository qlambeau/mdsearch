---
id: TASK-021
title: "Implement registered collection sources and source-aware updates"
type: implementation-tasks
status: implemented
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved implementation of the complete US-021 packet in chat on 2026-09-30"
parent: US-021
depends_on:
  - US-005
  - US-006
requires:
  - REQ-021
  - DES-021
  - DB-001
  - TABLE-013
blockers: []
related:
  - US-012
  - US-016
  - ADR-001
  - ADR-005
  - ADR-015
---

# Tasks

## Implementation Approach

Implement US-021 as one vertical slice within the existing domain, application,
filesystem, SQLite, and CLI layers. Write failing application, adapter, and
subprocess tests before production changes. Keep collection source persistence
and each collection's file/lexical/graph reconciliation atomic, while update-all
continues independently and aggregates failures. Do not add dependencies or
implement the later semantic/model-management and final CLI redesign slices.

## Ordered Tasks

- [x] **TASK-021-1 (RED):** Add failing application tests for registered-source discovery, overlapping-root deduplication, modifications and deletions, removed-source cleanup, legacy collections without sources, unreadable-file preservation, inaccessible-directory failure, and update-all continuation.
  - Depends on: None
  - Verification: Run the focused application test target and observe failures demonstrating absent source-aware update behavior.
- [x] **TASK-021-2 (RED):** Add failing SQLite tests for version-8 migration without inferred source rows, create/configure/list source persistence, canonical-path uniqueness, collection deletion cleanup, and rollback when lexical or graph rebuilding fails.
  - Depends on: TASK-021-1
  - Verification: Run the focused SQLite adapter tests and observe expected failures against the existing schema and persistence API.
- [x] **TASK-021-3 (RED):** Add failing CLI subprocess tests for create/configure/list in human and JSON formats, registration without indexing, discovery after a separate process invocation, source-less legacy guidance, and update-all reporting/exit status.
  - Depends on: TASK-021-1
  - Verification: Run the focused CLI integration tests and observe failures for the unimplemented commands and output contract.
- [x] **TASK-021-4 (GREEN):** Implement the domain source value, filesystem canonicalization/expansion behavior, source persistence and collection use cases, and source-aware update classification through existing ports.
  - Depends on: TASK-021-1
  - Verification: Application and domain tests pass; unreadable files with skip enabled retain existing content, while incomplete directory scans fail before reconcile.
- [x] **TASK-021-5 (GREEN):** Implement the schema-v8 migration, SQLite source CRUD/listing and cleanup, and transactional file/index reconciliation coverage for all supported migrations.
  - Depends on: TASK-021-2, TASK-021-4
  - Verification: SQLite adapter tests pass, including migration coverage from versions 0–7 and rollback assertions for file, lexical, and graph state.
- [x] **TASK-021-6 (GREEN):** Wire collection create/configure/list and update/update-all through the CLI; render source summaries and per-collection outcomes; update README command documentation.
  - Depends on: TASK-021-3, TASK-021-4, TASK-021-5
  - Verification: CLI subprocess tests pass, including JSON source fields, cross-process discovery, and nonzero aggregate update-all status after continuing through failures.
- [x] **TASK-021-7 (VERIFY):** Trace every approved scenario to tests and run repository and feature verification gates; record observed evidence.
  - Depends on: TASK-021-6
  - Verification: `cargo xtask validate-specs`, `cargo xtask ci`, and `cargo xtask eval` complete; all US-021 scenarios have passing evidence recorded below.

## Test And Verification Plan

- [x] Domain and application behavior tests cover source types, discovery, overlap, changes, removals, unreadable files, inaccessible roots, legacy collections, and update-all partial failure.
- [x] SQLite integration tests cover migrations 0–7 to schema 8, source persistence/uniqueness, cleanup, and atomic rollback of files and lexical/graph indexes.
- [x] CLI subprocess tests cover human/JSON listing, create/configure without indexing, later discovery, legacy guidance, and aggregate reports/exit status.
- [x] Gherkin scenarios: `scenarios.feature` (all scenarios traced to tests).
- [x] Quality gates: `cargo xtask validate-specs`, `cargo xtask ci`, and `cargo xtask eval`.
- [x] Non-functional checks: no new dependencies; additive migration preserves searchable legacy data and creates no inferred source records.

### Observed Verification Evidence (2026-09-30)

- Base commit: `6cc9227eefb22121ac1e67b5a117b8fee7e9cfd5`; implementation is in the current uncommitted worktree.
- RED: the new domain test initially failed to compile because `CollectionSource` and `SourceKind` did not exist. The new SQLite source integration test initially failed to compile because the source-store persistence API did not exist. In an isolated checkout of the base commit, `CARGO_TARGET_DIR=... cargo test -p kv-app --test collection_sources --test update_collection` also failed both source-registration CLI tests: the old `collection create` rejected the source path as an unknown argument. The source APIs and CLI support were implemented afterward; focused app, application, and SQLite suites passed.
- `cargo xtask validate-specs`: passed; scanned 116 artifacts.
- `cargo xtask ci`: passed all formatting, lint, tests, documentation, dependency, spec, and coverage gates. Total line coverage 87.64%, above the 85% threshold.
- `cargo xtask eval --verify-only`: passed; verified 32 documents, 32 queries, and 78 judgments.
- `cargo xtask eval`: passed all thresholds across 32 queries; Recall@5 1.0000, MRR@5 1.0000, NDCG@5 1.0000.
- `git diff --check`: passed.

## Rollout And Recovery

### Rollout

Schema version 8 adds the source table without changing legacy collection rows.
Existing collections remain searchable and must have sources explicitly
registered before update. New registration stores canonical paths but does not
index content until update. Each collection update commits discovered file
changes and lexical/graph indexes together. `update --all` reports each result
and continues after individual failures.

### Recovery

A failed create/configure transaction preserves the prior collection/source
configuration. A failed source scan or index rebuild preserves that collection's
last committed files and indexes; users can correct permissions or configuration
and retry. Skipped unreadable files retain their previous stored content. In
update-all, successful collections remain committed even if another collection
fails, and the aggregate command reports failure for retry.

## Definition Of Done

- [x] All ordered tasks are complete with observed RED before GREEN evidence.
- [x] All approved US-021 scenarios pass and trace to tests.
- [x] `cargo xtask validate-specs`, `cargo xtask ci`, and `cargo xtask eval` pass with observed output recorded.
- [x] Migration, rollback, partial failure, legacy behavior, and source listing are documented and verified.
- [x] Task verification evidence includes date, commit/worktree state, commands, and outcomes.
