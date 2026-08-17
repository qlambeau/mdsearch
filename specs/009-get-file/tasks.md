---
id: TASK-009
title: "Retrieve a complete file by name or ID implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-009
related:
  - REQ-009
  - DES-009
  - US-004
  - US-006
---

# Tasks

## Implementation Approach

Implement the smallest complete path for `mdsearch get`: a `FileId` domain value,
a read-only `FileRetrievalStore` port and `GetFile` use case that resolve a name
or ID to one stored file, a read-only `SqliteFileRetrievalStore` over the
existing `files` table, and the `get` CLI command printing raw content. Keep
`REQ-009` and the state flow in `DES-009` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces no dependency, schema change, migration, or workspace
member. It does not implement JSON output, related-concept links, or
cross-collection retrieval.

## Ordered Tasks

- [x] **TASK-009-1:** Add the `FileId(u64)` domain newtype with positive-value
      validation.
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; positive IDs are accepted,
    zero is rejected, and values round-trip.

- [x] **TASK-009-2:** Add the `FileRetrievalStore` port (`RetrievedFile`,
      `get_by_path`, `get_by_id`, `list_by_basename`), the `GetFile` use case,
      and `GetFileError` / `FileRetrievalStoreError`; add an in-memory fake for
      tests.
  - Depends on: TASK-009-1
  - Verification: `cargo test -p kv-application` passes; tests cover path,
    unique basename, ID, ambiguous basename, not-found by name, not-found by ID,
    and collection-not-found.

- [x] **TASK-009-3:** Implement the read-only `SqliteFileRetrievalStore` (open
      without DDL) for the three primitives against the `files` table.
  - Depends on: TASK-009-2
  - Verification: Store integration tests confirm path, ID, and basename
    lookups, including ambiguity and not-found.

- [x] **TASK-009-4:** Wire `mdsearch get COLLECTION NAME_OR_ID` with
      `--database PATH`; print the raw content and report clear errors,
      including a non-UTF-8 content error.
  - Depends on: TASK-009-3
  - Verification: CLI acceptance tests mapped from `scenarios.feature` confirm
    raw-content output and every error boundary.

- [x] **TASK-009-5:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-009-4
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [ ] Unit checks: `FileId` validation and round-trip.
- [ ] Application checks: `GetFile` use case with a fake store.
- [ ] Integration checks: path, ID, and basename lookups.
- [ ] CLI checks: raw-content output, ambiguity, not-found, missing collection,
      and missing database.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: read-only retrieval, offline execution, and no new
      workspace members or dependencies.
- [ ] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [ ] Regression check: confirm no JSON, link, or cross-collection retrieval
      behavior is added.

## Rollout And Recovery

### Rollout

Ship the `mdsearch get` command in the single compiled binary. It is read-only
and requires no schema migration.

### Recovery

A failed retrieval writes nothing and mutates no state; re-running with corrected
arguments retries safely. A missing database fails without creating a file.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope JSON, link, or cross-collection retrieval behavior was added.
- [x] Operational or documentation changes are complete.