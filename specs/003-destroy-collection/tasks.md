---
id: TASK-003
title: "Destroy a named collection implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: US-003
related:
  - REQ-003
  - DES-003
  - US-001
  - US-002
---

# Tasks

## Implementation Approach

Implement the smallest complete destroy path for `mdsearch`'s existing
collection store. Keep the behavior in `REQ-003` and the state flow in `DES-003`
authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice adds a `destroy_collection` operation to the existing port and
`SQLite` adapter, a `DestroyCollection` use case, and the `collection destroy`
command. It does not implement list, create, file ingestion, indexing, or JSON
output, and it introduces no schema migration or new dependency.

## Ordered Tasks

- [x] **TASK-003-1:** Extend the collection-store port and errors test-first.
  - Depends on: None
  - Verification: `cargo test -p kv-application` and `cargo test -p kv-app`
    compile and pass; the `DestroyCollection` use case is covered by an
    in-memory fake that removes by case-insensitive key, and error mapping is
    unit-tested.

- [x] **TASK-003-2:** Implement the `SQLite` destroy path.
  - Depends on: TASK-003-1
  - Verification: Integration tests confirm case-insensitive delete returns the
    retained spelling, `CollectionNotFound` for an absent name, `DatabaseNotFound`
    for a missing file without creating it, and other collections remaining
    intact.

- [x] **TASK-003-3:** Wire the `mdsearch collection destroy` command and output.
  - Depends on: TASK-003-2
  - Verification: CLI checks confirm the happy path, case-insensitive match,
    non-existent collection, missing database, invalid names, the `--database`
    override, and removal reflected in a later `collection list`.

- [x] **TASK-003-4:** Execute the approved Gherkin scenarios and the Rust
  constitution gates.
  - Depends on: TASK-003-3
  - Verification: Every scenario in `scenarios.feature` passes against isolated
    databases, and `cargo xtask ci` passes with observed output (fmt, clippy
    with `-D warnings`, test, doc, deny, and `llvm-cov` with the line-coverage
    threshold).

## Test And Verification Plan

- [x] Unit checks: error mapping for missing collection and missing database.
- [x] Application checks: `DestroyCollection` use case with an in-memory fake.
- [x] Integration checks: case-insensitive delete, retained spelling, missing
      collection, missing database, and isolation of other collections.
- [x] CLI checks: command parsing, human-readable confirmation, and semantic
      failure output.
- [x] Gherkin scenarios: `scenarios.feature`.
- [x] Non-functional checks: offline execution and no server dependency.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates in
      `specs/CONSTITUTION.md`.
- [x] Regression check: confirm no list, create, ingestion, indexing, or JSON
      behavior is introduced by this slice.

## Rollout And Recovery

### Rollout

Ship the `collection destroy` command in the single compiled binary. It writes
to the existing database and requires no migration.

### Recovery

The operation is destructive by design and offers no undo. A missing database
is reported without creating a file, a missing collection is reported without
modifying the database, and an invalid name is rejected before any database
access. Retrying a failed destroy is safe.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope list, create, ingestion, indexing, or JSON behavior was added.
- [x] Operational or documentation changes are complete.
