---
id: TASK-001
title: "Create an empty named collection implementation tasks"
type: implementation-tasks
status: draft
created: 2026-08-11
updated: 2026-08-11
owner: TBD
parent: US-001
related:
  - REQ-001
  - DES-001
  - ADR-001
---

# Tasks

## Implementation Approach

Implement the smallest complete Rust CLI path for creating an empty persistent
collection. Keep the behavior in `REQ-001`, the state flow in `DES-001`, and the
database decision in `ADR-001` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice includes the SQLite collection foundation, database-path handling,
name validation, transactional persistence, CLI output, and acceptance tests.
It does not implement file ingestion, lexical indexing, vectorization with
`fastembed`, semantic search, entity extraction, or the `async-graphql` context
schema.

## Ordered Tasks

- [ ] **TASK-001:** Establish the Rust single-binary and SQLite foundation.
  - Depends on: None
  - Verification: The binary builds with the selected SQLite integration; the schema/migration mechanism is available; the build can validate or load the approved `sqlite-vector` extension without requiring a server.

- [ ] **TASK-002:** Implement database-path resolution and initialization.
  - Depends on: TASK-001
  - Verification: Unit and integration checks cover the default `~/.kv/collections.db` path, the `--database PATH` override, missing parent-directory creation, and semantic failure when the path cannot be resolved or accessed.

- [ ] **TASK-003:** Implement collection-name normalization and validation.
  - Depends on: None
  - Verification: Unit checks cover trimming, empty and whitespace-only names, `/`, `\\`, control characters, no product-defined maximum length, Unicode-aware case folding, and preservation of the trimmed display spelling.

- [ ] **TASK-004:** Implement the versioned collection schema and transactional repository operation.
  - Depends on: TASK-001, TASK-002, TASK-003
  - Verification: Integration checks confirm the collection fields, unique comparison key, empty initial state, persistence, duplicate rejection, and rollback without partial collection state.

- [ ] **TASK-005:** Wire the `kv collection create NAME` command and optional `--database PATH` flag.
  - Depends on: TASK-002, TASK-003, TASK-004
  - Verification: CLI checks confirm successful creation, retained-name confirmation, semantic invalid-name and duplicate failures, database failure reporting, and no behavior outside `REQ-001`.

- [ ] **TASK-006:** Execute the approved Gherkin acceptance scenarios.
  - Depends on: TASK-005
  - Verification: Every scenario and every example in `scenarios.feature` passes against isolated databases, including persistence across separate CLI invocations.

- [ ] **TASK-007:** Run final build, offline, and recovery checks.
  - Depends on: TASK-006
  - Verification: The release binary builds, the create flow works without network access, the selected SQLite/vector foundation is packaged as designed, and failed operations leave no partial collection.

- [ ] **TASK-008:** Execute the Rust constitution's required tooling and Definition of Done gates.
  - Depends on: TASK-007
  - Verification: `cargo xtask ci` and the applicable `cargo fmt`, `cargo clippy`, `cargo test`, `cargo doc`, `cargo deny`, and coverage gates from `specs/CONSTITUTION.md` pass with observed output.

## Test And Verification Plan

- [ ] Unit checks: path resolution, name normalization, invalid-character validation, Unicode case folding, and domain error mapping.
- [ ] Integration checks: schema initialization, custom database paths, persistence, uniqueness, transaction rollback, and database-access failures.
- [ ] CLI checks: command parsing, human-readable success output, and semantic failure output.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: Rust release build, offline execution, SQLite/vector-extension availability, and no server dependency.
- [ ] Constitution checks: all applicable rules in `specs/CONSTITUTION.md` are satisfied and the required tooling gates pass.
- [ ] Regression check: confirm no ingestion, search, vectorization, entity extraction, or GraphQL server behavior is introduced by this slice.

## Rollout And Recovery

### Rollout

Ship the collection-create command in the single compiled binary. On first use,
initialize the selected SQLite database and apply the versioned collection
schema. Existing databases must be migrated only through the planned schema
migration mechanism.

### Recovery

If validation fails, do not open or mutate the database. If initialization or
the collection write fails, roll back the transaction and report the semantic
failure. An empty database file may remain after schema initialization fails,
but no partial collection may remain. Retrying the command must be safe.

## Definition Of Done

- [ ] All tasks are complete.
- [ ] Automated unit, integration, and CLI checks pass.
- [ ] The executable scenarios pass.
- [ ] The release binary builds with the approved Rust and SQLite foundation.
- [ ] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [ ] Offline and no-server constraints are verified.
- [ ] Relevant specifications are updated if implementation details require clarification.
- [ ] No out-of-scope ingestion, indexing, vectorization, entity, or GraphQL server behavior was added.
- [ ] Operational or documentation changes are complete.
