---
id: TASK-007
title: "Search the lexical index for ranked passages implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-007
related:
  - REQ-007
  - DES-007
  - US-006
  - ADR-001
---

# Tasks

## Implementation Approach

Implement the smallest complete path for `mdsearch search`: a domain
`PassageKind::from_key` reconstructor, a `LexicalSearchStore` port and
`SearchLexical` use case, a read-only `SqliteLexicalSearchStore` that queries
the built FTS5 index with `bm25()` ranking, and the `search` CLI command with
`--collection`, `--limit`, and `--database`. Keep `REQ-007` and the state flow
in `DES-007` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces no dependency, schema change, migration, or workspace
member. It does not implement JSON output, diff-style positions, file retrieval,
related-concept links, or semantic or contextual retrieval.

## Ordered Tasks

- [x] **TASK-007-1:** Add `PassageKind::from_key` to the domain passage module.
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; every kind round-trips from
    its stable key and unknown keys return `None`.

- [x] **TASK-007-2:** Add the `LexicalSearchStore` port (`SearchResult`,
      `SearchResultSet`, `SearchScope`), the `SearchLexical` use case, and
      `SearchError` / `SearchStoreError`; add an in-memory fake for tests.
  - Depends on: TASK-007-1
  - Verification: `cargo test -p kv-application` passes; tests cover all and
    collection scopes, empty output, empty-query rejection, and store-error
    propagation.

- [x] **TASK-007-3:** Implement the read-only `SqliteLexicalSearchStore`
      (open without DDL) that runs `passages MATCH`, ranks with `-bm25()`,
      orders deterministically, applies the limit, reports the total via
      `COUNT(*) OVER()`, maps FTS5 syntax errors to `InvalidQuery`, and treats
      pre-v3 databases as having no built collections.
  - Depends on: TASK-007-2
  - Verification: Store integration tests confirm ranking order, deterministic
    tie-breaking, limit and total, collection restriction, unknown and unbuilt
    collections, pre-v3 databases, malformed queries, and exact-phrase matches.

- [x] **TASK-007-4:** Wire `mdsearch search QUERY` with `--collection NAME`,
      `--limit N` (range 1 through 100, default 10), and `--database PATH`;
      render result blocks and the total-count summary.
  - Depends on: TASK-007-3
  - Verification: CLI acceptance tests mapped from `scenarios.feature` confirm
    rendering, `--limit` bounds, `--collection`, empty and malformed queries,
    empty output, and the missing-database boundary.

- [x] **TASK-007-5:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-007-4
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [ ] Unit checks: `PassageKind::from_key` round-trips and unknown-key rejection.
- [ ] Application checks: `SearchLexical` use case with a fake store.
- [ ] Integration checks: ranking, tie-breaking, limit/total, scope,
      malformed-query, and pre-v3 behavior.
- [ ] CLI checks: rendering, `--limit` bounds, `--collection`, empty/malformed
      queries, empty output, and missing database.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: read-only behavior, offline execution, and no new
      workspace members or dependencies.
- [ ] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [ ] Regression check: confirm no JSON output, positions, file retrieval,
      related-concept links, or semantic/contextual behavior is added.

## Rollout And Recovery

### Rollout

Ship the `mdsearch search` command in the single compiled binary. It is
read-only and requires no schema migration; it works against any database that
has been migrated to schema version 3 by a previous `collection update`.

### Recovery

A failed search writes nothing and mutates no state; re-running with corrected
arguments retries safely. A missing database fails without creating a file. A
database at schema version 2 or older reports no built collections (empty output
when searching all, a clear not-built error when a collection is targeted).

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope JSON, position, file-retrieval, link, or semantic/contextual behavior was added.
- [x] Operational or documentation changes are complete.