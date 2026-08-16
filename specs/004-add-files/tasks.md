---
id: TASK-004
title: "Add markdown files to a collection implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-004
related:
  - REQ-004
  - DES-004
  - TABLE-003
  - DB-001
---

# Tasks

## Implementation Approach

Implement the smallest complete ingestion path for `mdsearch`: add `sha2`
hashing in `domain`, `FileSystem` and `FileStore` ports in `application`, an
`AddFiles` use case, a versioned migration plus `files` table in the `SQLite`
adapter, a `SystemFileSystem` in `infrastructure`, and the `collection add`
command. Keep the behavior in `REQ-004` and the state flow in `DES-004`
authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces the approved `sha2` dependency only. It does not implement
the update command, lexical, semantic, or entity indexing, retrieval, JSON
output, or frontmatter parsing, and it adds no new workspace member.

## Ordered Tasks

- [x] **TASK-004-1:** Add `sha2` to the workspace and implement
      `domain::ContentHash`.
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; property tests confirm
    identical content hashes identically and differing content hashes
    differently.

- [x] **TASK-004-2:** Add `FileSystem` and `FileStore` ports, `FileRecord`, and
      error types to `application`, each with an in-memory fake.
  - Depends on: None
  - Verification: `cargo test -p kv-application` compiles and passes; fakes are
    exercised by contract-style tests.

- [x] **TASK-004-3:** Implement the versioned migration, `files` table, and
      `SqliteFileStore::upsert_files` in the `SQLite` adapter, plus
      `SystemFileSystem` in `infrastructure`.
  - Depends on: TASK-004-1, TASK-004-2
  - Verification: Integration tests confirm migration to version 2, upsert
    idempotency by path, retained `file_id` and `created_at`, collection-not-found,
    and filesystem walk/read/canonicalization/extension filtering.

- [x] **TASK-004-4:** Implement the `AddFiles` use case.
  - Depends on: TASK-004-2
  - Verification: Use-case tests with fakes cover success, recursion,
    non-`.md` ignoring, upsert, atomic failure, and `--force` skip.

- [x] **TASK-004-5:** Wire the `mdsearch collection add` command with
      `--database` and `--force`.
  - Depends on: TASK-004-3, TASK-004-4
  - Verification: CLI checks confirm counts, `--force` skips, missing collection,
    missing database, and the `--database` override.

- [x] **TASK-004-6:** Create `TABLE-003` and update `DB-001` for the `files`
      table and schema version 2.
  - Depends on: TASK-004-3
  - Verification: Frontmatter and references are consistent and bidirectional.

- [x] **TASK-004-7:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-004-5, TASK-004-6
  - Verification: Every scenario in `scenarios.feature` passes against isolated
    databases, and `cargo xtask ci` passes with observed output (fmt, clippy
    with `-D warnings`, test, doc, deny, and `llvm-cov` with the line-coverage
    threshold).

## Test And Verification Plan

- [x] Unit checks: `ContentHash` determinism and error mapping.
- [x] Application checks: `AddFiles` use case with fakes.
- [x] Integration checks: migration, upsert idempotency, retained identity, and
      filesystem discovery.
- [x] CLI checks: command parsing, counts, skip reporting, and semantic failures.
- [x] Gherkin scenarios: `scenarios.feature`.
- [x] Non-functional checks: offline execution and no server dependency.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates in
      `specs/CONSTITUTION.md`.
- [x] Regression check: confirm no update, indexing, retrieval, or JSON behavior
      is introduced by this slice.

## Rollout And Recovery

### Rollout

Ship the `collection add` command in the single compiled binary. On first add,
the existing database is migrated to schema version 2 by creating the `files`
table. Existing collections remain unchanged.

### Recovery

Without `--force`, a failed add writes nothing, so retrying is safe. With
`--force`, unreadable paths are skipped and reported. Upserts are idempotent by
canonical path.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope update, indexing, retrieval, or JSON behavior was added.
- [x] Operational or documentation changes are complete.
