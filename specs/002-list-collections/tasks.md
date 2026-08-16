---
id: TASK-002
title: "List all collections implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-16
updated: 2026-08-16
owner: TBD
parent: US-002
related:
  - REQ-002
  - DES-002
  - US-001
---

# Tasks

## Implementation Approach

Implement the smallest complete read-only list path for `mdsearch`'s existing
collection store. Keep the behavior in `REQ-002` and the state flow in `DES-002`
authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice adds a read-only `list_collections` operation to the existing port
and `SQLite` adapter, a `ListCollections` use case, and the `collection list`
command. It does not implement create, destroy, file ingestion, indexing, or
JSON output, and it introduces no schema migration or new dependency.

## Ordered Tasks

- [x] **TASK-002-1:** Extend the collection-store port and errors test-first.
  - Depends on: None
  - Verification: `cargo test -p kv-application` and `cargo test -p kv-app`
    compile and pass; the `ListCollections` use case is covered by an in-memory
    fake, and error mapping is unit-tested.

- [x] **TASK-002-2:** Implement the read-only `SQLite` list path.
  - Depends on: TASK-002-1
  - Verification: Integration tests confirm `open_existing` returns
    `DatabaseNotFound` for a missing file without creating it, an empty database
    lists no names, multiple names are returned case-insensitively sorted, and an
    inaccessible database fails semantically.

- [x] **TASK-002-3:** Wire the `mdsearch collection list` command and output.
  - Depends on: TASK-002-2
  - Verification: CLI checks confirm the happy path, empty output for an empty
    database, missing-database failure without file creation, the `--database`
    override, persistence across runs, and no output beyond the requested names.

- [x] **TASK-002-4:** Execute the approved Gherkin scenarios and the Rust
  constitution gates.
  - Depends on: TASK-002-3
  - Verification: Every scenario in `scenarios.feature` passes against isolated
    databases, and `cargo xtask ci` passes with observed output (fmt, clippy
    with `-D warnings`, test, doc, deny, and `llvm-cov` with the line-coverage
    threshold).

## Test And Verification Plan

- [x] Unit checks: error mapping for missing and inaccessible databases.
- [x] Application checks: `ListCollections` use case with an in-memory fake.
- [x] Integration checks: read-only open, ordering, empty result, missing and
      inaccessible database behavior.
- [x] CLI checks: command parsing, human-readable output, empty output, and
      semantic failure output.
- [x] Gherkin scenarios: `scenarios.feature`.
- [x] Non-functional checks: offline execution, read-only guarantee, and no
      server dependency.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates in
      `specs/CONSTITUTION.md`.
- [x] Regression check: confirm no create, destroy, ingestion, indexing, or
      JSON behavior is introduced by this slice.

## Rollout And Recovery

### Rollout

Ship the `collection list` command in the single compiled binary. It reads the
existing database and requires no migration. Existing databases remain
unchanged.

### Recovery

The operation is read-only: it performs no writes and no DDL. A missing database
is reported without creating a file, and an inaccessible database is reported
without modifying it. Retrying the command is always safe.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope create, destroy, ingestion, indexing, or JSON behavior was added.
- [x] Operational or documentation changes are complete.
