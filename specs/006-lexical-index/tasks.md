---
id: TASK-006
title: "Build the lexical index during collection update implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-006
related:
  - REQ-006
  - DES-006
  - ADR-005
  - US-005
  - DB-001
  - TABLE-004
  - TABLE-005
  - TABLE-006
---

# Tasks

## Implementation Approach

Implement the smallest complete path for building a per-passage lexical index
during `collection update` and observing it with `mdsearch index status`:
a domain `passage` module (lenient frontmatter extraction + paragraph split), an
extended `FileStore::reconcile` contract that rebuilds the index atomically, a
new `LexicalIndexStore` port and `IndexStatus` use case, a schema-v3 migration
with the FTS5 `passages` table plus `passage_files` and `lexical_index_state`,
and the `index status` CLI. Keep `REQ-006` and the state flow in `DES-006`
authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces the approved `yaml-rust2` dependency (ADR-005), the
schema-v3 migration (DB-001, TABLE-004/005/006), and the FTS5-based rebuild. It
does not implement the search command, JSON output, positions, or semantic or
contextual indexing.

## Ordered Tasks

- [x] **TASK-006-1:** Add `yaml-rust2` to `[workspace.dependencies]` and
      `crates/domain`; implement `Passage`, `PassageKind`, `FrontmatterIssue`,
      and `segment_passages` (lenient `title`/`tags`/`aliases`/`summary`
      extraction and blank-line paragraph split).
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; unit and property tests
    cover body-only files, all four frontmatter fields, absent and malformed
    frontmatter, empty files, and paragraph boundaries; malformed frontmatter
    yields `FrontmatterIssue::Malformed` with body-only passages.

- [x] **TASK-006-2:** Extend `FileStore::reconcile` to return
      `ReconcileOutcome { malformed_frontmatter }`; add `malformed_frontmatter`
      to `UpdateOutcome`; add the `LexicalIndexStore` port, `IndexStatus`,
      `IndexStoreError`, and the `IndexStatus` use case; update the in-memory
      fakes for the extended contracts.
  - Depends on: TASK-006-1
  - Verification: `cargo test -p kv-application` passes; `UpdateCollection`
    tests cover the malformed count and `IndexStatus` tests cover built,
    not-built, and empty-database outcomes.

- [x] **TASK-006-3:** Bump `CURRENT_SCHEMA_VERSION` to 3 and extend `migrate`
      with `passages` (FTS5), `passage_files`, and `lexical_index_state`;
      implement the atomic index rebuild inside `reconcile` (segment each stored
      file, delete the collection's stale rows, insert new rows, upsert index
      state); add the DDL-free `SqliteLexicalIndexStore` (schema below 3 reports
      `NotBuilt`, missing database returns `DatabaseNotFound`).
  - Depends on: TASK-006-2
  - Verification: Store integration tests confirm migration idempotency, rebuild
    counts, stale-row removal, atomic rollback on a forced index failure, and
    `NotBuilt` reporting on a schema-v2 database.

- [x] **TASK-006-4:** Wire `mdsearch index status` (optional `--database`) and
      append the malformed-frontmatter report to `collection update` output.
  - Depends on: TASK-006-3
  - Verification: CLI acceptance tests confirm per-collection status lines, empty
    output for no collections, missing database without file creation, and the
    update output's malformed report.

- [x] **TASK-006-5:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-006-4
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [x] Unit checks: `segment_passages` segmentation, frontmatter extraction, and
      malformed/absent/empty edge cases.
- [x] Application checks: `UpdateCollection` malformed count and `IndexStatus`
      use case with fakes.
- [x] Integration checks: v3 migration, rebuild counts, stale-row removal,
      atomic rollback, and schema-v2 `NotBuilt` reporting.
- [x] CLI checks: `index status` rendering, empty output, missing database, and
      the update output's malformed report.
- [x] Gherkin scenarios: `scenarios.feature`.
- [x] Non-functional checks: offline execution, no server dependency, FTS5
      availability in the bundled SQLite build, and no new workspace members.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [x] Regression check: confirm no search command, JSON output, positions, or
      semantic/contextual indexing behavior is added.

## Rollout And Recovery

### Rollout

Ship the schema-v3 migration and the `index status` command in the single
compiled binary. Migration is applied idempotently when a database is opened for
ingestion (`collection add`/`update`); existing schema-v2 databases gain the new
tables on the next `collection update`. `mdsearch index status` is read-only and
opens the database without DDL.

### Recovery

A failed `collection update` writes nothing: file changes and the index rebuild
commit or roll back together, and re-running the update retries safely. The
rebuild is idempotent because it always starts from the reconciled file set.
A database at schema version 2 or older reports every collection as `NotBuilt`
rather than failing. A missing database fails without creating a file.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope search, JSON, position, or semantic/contextual behavior was added.
- [x] Operational or documentation changes are complete.