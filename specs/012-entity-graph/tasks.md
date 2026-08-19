---
id: TASK-012
title: "Deterministic entity graph build and internal query layer implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: US-012
related:
  - REQ-012
  - DES-012
  - ADR-001
  - ADR-005
  - ADR-008
  - DB-001
  - US-004
  - US-006
---

# Tasks

## Implementation Approach

Implement the smallest complete EPIC-005 path in dependency order: first make
the schema contract and approved dependency explicit, then write failing domain,
port, storage, traversal, and CLI tests before their implementations. Keep
graph extraction pure and deterministic in `kv-domain`; keep graph reads behind
an application `GraphStore` port; keep SQLite writes inside the existing
per-collection `FileStore::reconcile` transaction; and expose only the approved
in-process `async_graphql` query layer plus the debug neighbor command.

All Rust work must obey `specs/CONSTITUTION.md`: no unsafe code, no unapproved
workspace members or dependencies, tests before implementation, spec IDs in
test names or comments, typed errors, documented public items, and the complete
`cargo xtask ci` gate before completion. This slice does not add LLM claim
extraction, public related-concept output to `search`/`hybrid`, or any unrelated
refactor.

## Ordered Tasks

- [x] **TASK-012-1:** Define the physical graph schema artifacts before code:
      create `TABLE-010` for `nodes`, `TABLE-011` for `edges`, and `TABLE-012`
      for `graph_state`; update `DB-001` with schema version 6, the table
      catalog, bidirectional table references, and the new related artifacts.
  - Depends on: None
  - Verification: The three table documents define the approved node types,
      edge types, stable identities, uniqueness/foreign-key constraints,
      per-collection state, rebuild invariants, and rollback expectations;
      `DB-001` references every table exactly once and every table references
      `DB-001`.

- [x] **TASK-012-2:** Add the approved `async-graphql` dependency to the
      workspace and `kv-app` only, using the compatible pinned version and
      minimal in-process features required by ADR-008; do not add a GraphQL
      server, a new workspace member, or another architectural layer.
  - Depends on: TASK-012-1
  - Verification: `cargo check --workspace` and `cargo deny check` validate the
      dependency, license, advisory, and feature choices; the dependency is
      absent from `domain`, `application`, and `store-sqlite` manifests.

- [x] **TASK-012-3:** Write the failing domain tests for `REQ-012` before adding
      graph production code. Cover file/tag/alias node identity, all five edge
      kinds, relative Markdown-link resolution, `related:`/`sources:` parsing,
      unresolved-reference skipping, tag/alias name collisions, deterministic
      ordering, and duplicate elimination.
  - Depends on: TASK-012-1
  - Verification: The new tests reference `REQ-012`/`FR-002`..`FR-013` and
      `scenarios.feature` names and fail for the expected missing graph API
      before implementation; no assertion is weakened to obtain the failure.

- [x] **TASK-012-4:** Implement the pure domain graph model and extractor:
      `EntityKind`, `RelationKind`, `GraphNode`, `GraphEdge`, `EntityGraph`, and
      `extract_graph`. Extend frontmatter handling for `related:` and `sources:`
      without changing existing passage behavior; scan approved relative
      Markdown links, resolve only known collection files, skip unresolved
      targets, and keep tag and alias nodes distinct.
  - Depends on: TASK-012-3
  - Verification: `cargo test -p kv-domain` passes; the tests from TASK-012-3
      are green, the extractor is deterministic for identical inputs, and
      existing passage parser tests remain green.

- [x] **TASK-012-5:** Write failing application contract tests for the graph
      read port. Cover node lookup, optional relation filtering, finite
      hop-limited traversal, cycle protection, unknown collection/node errors,
      and read-only behavior using an in-memory test double shape.
  - Depends on: TASK-012-4
  - Verification: Tests cite `REQ-012`/`FR-014`..`FR-016` and the corresponding
      Gherkin scenarios, and fail before the `GraphStore` port and fake exist.

- [x] **TASK-012-6:** Add the application `GraphStore` port, domain-language
      graph query/result types, typed `GraphStoreError`, and the required
      in-memory fake. Keep the port read-only and separate from the existing
      file/index write ports; document lookup, ordering, depth, cycle, and error
      contracts on the public items.
  - Depends on: TASK-012-5
  - Verification: `cargo test -p kv-application` passes; contract tests cover
      every port operation and error path, and `cargo doc -p kv-application
      --no-deps` emits no new documentation errors.

- [x] **TASK-012-7:** Write failing SQLite integration tests for schema-v6
      migration and graph rebuild behavior. Cover initial build, every node and
      edge type, unresolved links, empty collections, tag/alias collisions,
      idempotent update, deleted-file cleanup, preservation of lexical/semantic
      behavior, and a forced graph-write failure that must roll back the full
      collection transaction.
  - Depends on: TASK-012-4, TASK-012-6
  - Verification: Store tests cite `REQ-012`/`FR-001`..`FR-013` and `FR-018` plus
      the relevant Gherkin scenarios; they fail before schema-v6 and
      `rebuild_graph` are implemented.

- [x] **TASK-012-8:** Implement schema-v6 migration and integrate
      `rebuild_graph` into `SqliteFileStore::reconcile` after the lexical
      rebuild and before commit. Read the current stored file set, call the
      pure extractor, replace only the target collection's graph, update
      `graph_state`, preserve the prior graph on failure, and keep all existing
      file/lexical/semantic behavior unchanged.
  - Depends on: TASK-012-7
  - Verification: `cargo test -p kv-store-sqlite` passes; migration works for
      existing schema-v5 databases, graph rebuild tests are green, deleted-file
      nodes/edges disappear, unchanged updates create no duplicates, empty
      collections succeed, and rollback leaves the prior graph intact.

- [x] **TASK-012-9:** Write failing SQLite graph-query integration tests for
      node lookup, direct neighbors, relation filtering, depth reporting,
      one-hop truncation, cycle termination, deterministic result ordering,
      unknown node/collection behavior, and empty-neighbor results.
  - Depends on: TASK-012-8
  - Verification: Tests cite `REQ-012`/`FR-014`..`FR-016` and the query-layer
      scenarios and fail before the recursive-CTE read implementation exists.

- [x] **TASK-012-10:** Implement `SqliteGraphStore` over schema-v6. Use bound
      SQL parameters and indexed node/edge lookups; implement node lookup,
      optional relation filtering, and recursive-CTE traversal with a finite
      hop limit, cycle guard, stable depth, and deterministic ordering. Do not
      expose raw SQLite or GraphQL types through the application port.
  - Depends on: TASK-012-9
  - Verification: `cargo test -p kv-store-sqlite` passes; the tests from
      TASK-012-9 are green, queries are read-only, and a representative 1-3 hop
      traversal remains within the PRD's soft harness-latency target.

- [x] **TASK-012-11:** Write failing in-process query-layer and CLI acceptance
      tests. Exercise the approved `async_graphql` schema/resolvers over the
      graph port and `mdsearch graph neighbors ID` with collection/database
      selection, relation/depth rendering, empty neighbors, unknown node,
      unknown collection, and missing-database errors.
  - Depends on: TASK-012-2, TASK-012-10
  - Verification: App tests cite `REQ-012`/`FR-014`..`FR-017` and the CLI/query
      Gherkin scenarios and fail before resolver and command wiring exists.

- [x] **TASK-012-12:** Implement the internal `async_graphql` query layer and
      wire the debug `mdsearch graph neighbors ID` command into the existing
      CLI composition root. Use the `GraphStore` port, keep the command
      read-only, honor `--collection NAME` and `--database PATH`, and map typed
      failures without changing the public `search`/`hybrid` output.
  - Depends on: TASK-012-11
  - Verification: `cargo test -p kv-app` passes; CLI and resolver tests are
      green, output includes neighbor identity/relation/depth, missing database
      does not create a file, and no GraphQL server or network access is added.

- [x] **TASK-012-13:** Execute the complete acceptance and constitution
      verification pass, then review the implementation against the approved
      packet. Run every scenario in `scenarios.feature`, verify all
      `REQ-012` functional requirements and failure paths, check existing
      commands for regressions, and capture observed gate output.
  - Depends on: TASK-012-12
  - Verification: `cargo xtask ci` passes with observed output for formatting,
      clippy with `-D warnings`, workspace tests, docs, `cargo deny`, and
      coverage; all Gherkin scenarios pass; schema links/statuses and ADR/design
      traceability are current.

## Test And Verification Plan

- [x] Domain unit checks: graph value types, node identity, all five edge types,
      frontmatter fields, relative-link extraction, unresolved references,
      deterministic ordering, idempotency, and tag/alias separation.
- [x] Application checks: `GraphStore` contract and fake for lookup, relation
      filtering, depth, cycle guard, unknown node/collection, and read-only
      behavior.
- [x] SQLite integration checks: schema-v6 migration, graph build in
      `reconcile`, full replacement, stale-node cleanup, empty graph, duplicate
      prevention, transaction rollback, and preservation of existing indexes.
- [x] Query/CLI checks: in-process `async_graphql` resolvers and
      `mdsearch graph neighbors ID` output, scope, depth/relation data, and
      database/error boundaries.
- [x] Gherkin scenarios: every scenario in `scenarios.feature`.
- [x] Non-functional checks: no network or LLM access, bound SQL parameters,
      read-only graph queries, deterministic output, migration compatibility,
      and representative 1-3 hop traversal latency.
- [x] Constitution checks: tests-first red-to-green evidence, traceable test
      names/comments, public documentation, dependency/layering rules, and
      `cargo xtask ci` with the Definition of Done checklist.
- [x] Regression checks: `search`, `hybrid`, `embed`, `get`, `add`, collection
      lifecycle, and lexical index behavior remain unchanged except for the
      approved graph side effect of `update`.

## Rollout And Recovery

### Rollout

Ship schema v6 and the graph capability in the single compiled binary. Existing
databases migrate forward by adding graph tables; existing collections receive
their graph on the next successful `mdsearch update`. No network or model asset
is required. `async-graphql` is compiled into the app as an internal dependency;
no server endpoint or new runtime service is deployed.

### Recovery

Schema migration must be idempotent and must not rewrite existing file, lexical,
or semantic data. Each collection's file, lexical, graph, and graph-state
changes commit together; a graph extraction or write failure rolls back that
collection and leaves its previous graph available for retry. For all-collection
updates, a failed collection is retried by rerunning its targeted update without
rebuilding already committed collections. A failed dependency/license gate
blocks the build before release and does not alter runtime data. A failed or
missing graph query target is read-only and reports an error without creating or
modifying the database.

## Definition Of Done

- [x] All ordered tasks are complete and their verification checks are observed.
- [x] `TABLE-010`..`TABLE-012` and `DB-001` schema references are complete and
      consistent.
- [x] Domain, application, store, query-layer, and CLI tests pass, including
      every specified error and recovery path.
- [x] Every scenario in `scenarios.feature` passes.
- [x] `cargo xtask ci` passes cleanly with the required coverage threshold.
- [x] Existing commands and indexes have no regression.
- [x] Offline, no-LLM, read-only query, migration, and rollback constraints are
      verified.
- [x] Approved specifications and ADR traceability remain current.
- [x] No EPIC-006 related-links output, LLM claim extraction, or speculative
      graph behavior was added.
