---
id: TASK-005
title: "Update a collection implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-005
related:
  - REQ-005
  - DES-005
  - US-004
---

# Tasks

## Implementation Approach

Implement the smallest complete reconciliation path for `mdsearch`: a
`ContentHash::try_from_hex` constructor, `FileStore::list_files` and
`FileStore::reconcile` plus `FileSystem::exists`, an `UpdateCollection` use
case, and the `collection update` command with `--all`. Keep the behavior in
`REQ-005` and the state flow in `DES-005` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces no new dependency, schema change, or workspace member. It
does not implement indexing, retrieval, JSON output, or frontmatter parsing.

## Ordered Tasks

- [x] **TASK-005-1:** Add `ContentHash::try_from_hex` and `ContentHashError`.
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; hex round-trips and invalid
    input is rejected.

- [x] **TASK-005-2:** Extend `FileStore` with `list_files`/`reconcile` and
      `FileSystem` with `exists`; add `StoredFile`, `UpdateCollection`,
      `UpdateOutcome`, `UpdateTarget`, and `UpdateCollectionError`.
  - Depends on: TASK-005-1
  - Verification: `cargo test -p kv-application` compiles and passes.

- [x] **TASK-005-3:** Implement `list_files` and `reconcile` in
      `SqliteFileStore`, and `exists` in `SystemFileSystem`.
  - Depends on: TASK-005-2
  - Verification: Integration tests confirm listing, atomic upsert+delete,
      collection-not-found, and existence semantics.

- [x] **TASK-005-4:** Wire `collection update NAME PATH...` and `--all`.
  - Depends on: TASK-005-3
  - Verification: CLI checks confirm added/modified/deleted counts, `--all`,
      `--force`, missing collection/database, and the `--database` override.

- [x] **TASK-005-5:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-005-4
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [x] Unit checks: `ContentHash` reconstruction and error mapping.
- [x] Application checks: `UpdateCollection` use case with fakes.
- [x] Integration checks: list/reconcile and existence semantics.
- [x] CLI checks: counts, `--all`, `--force`, and semantic failures.
- [x] Gherkin scenarios: `scenarios.feature`.
- [x] Non-functional checks: offline execution and no server dependency.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [x] Regression check: confirm no indexing, retrieval, or JSON behavior is added.

## Rollout And Recovery

### Rollout

Ship the `collection update` command in the single compiled binary. It reuses
the schema from `004-add-files` and requires no migration.

### Recovery

Without `--force`, a failed update writes nothing. With `--force`, unreadable
paths are skipped and reported. Reconcile is idempotent by canonical path.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope indexing, retrieval, or JSON behavior was added.
- [x] Operational or documentation changes are complete.
