---
id: TASK-013
title: "Context recovery from the entity graph implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: US-013
related:
  - REQ-013
  - DES-013
  - ADR-008
  - DEC-012
  - US-012
---

# Tasks

## Implementation Approach

Implement the smallest complete EPIC-007 path in dependency order: write failing
tests for the related-enrichment helper, its rendering, and the `mdsearch
context` command before their implementations, then wire them into the existing
app composition root. Reuse the EPIC-005 `GraphStore` port, `SqliteGraphStore`,
and in-process `async_graphql` schema; add no crate, workspace member, layer, or
schema change. `--related` only adds per-result file-to-file context and never
changes ranked results; `mdsearch context` is read-only and requires
`--collection`.

All Rust work must obey `specs/CONSTITUTION.md`: no unsafe code, no unapproved
workspace members or dependencies, tests before implementation, spec IDs in
test names or comments, typed errors, documented public items, and the complete
`cargo xtask ci` gate before completion. This slice does not add graph build
behavior, LLM/claim extraction, external services, or a GraphQL server.

## Ordered Tasks

- [x] **TASK-013-1:** Write failing unit tests for the related-enrichment helper
      before adding it: resolving a result file's `NodeId` from its path,
      keeping only File-destination neighbors, restricting relations to the
      closed set (`LINKS_TO`, `RELATED_TO`, `HAS_SOURCE`), deduplication, and an
      empty result when the node or graph is missing.
  - Depends on: None
  - Verification: Tests cite `REQ-013`/`FR-001`, `FR-002`, `FR-012` and the
      corresponding Gherkin scenarios, and fail for the missing helper before
      implementation; no assertion is weakened to obtain the failure.

- [x] **TASK-013-2:** Implement the related-enrichment helper over the
      `GraphStore` port: a single hop-1 `neighbors` call per result, filtered to
      File destinations and the closed file-to-file relation set, deduplicated,
      returning `(PathBuf, RelationKind)` pairs; a missing node or graph yields
      an empty set, never a failure.
  - Depends on: TASK-013-1
  - Verification: `cargo test -p kv-app` passes; the TASK-013-1 tests are green
      and existing app tests remain green.

- [x] **TASK-013-3:** Write failing rendering tests for `--related` output:
      human lines `related: <path> (<RELATION>)` (one per related file, none for
      results without related files) and the JSON `related` field as a structured
      array of `{ "path", "relation" }` objects that is additive to the existing
      JSON shape; ranked results are unchanged with or without the switch.
  - Depends on: TASK-013-2
  - Verification: Tests cite `REQ-013`/`FR-002`, `FR-003`, `FR-004` and the
      related scenarios and fail before rendering is wired.

- [x] **TASK-013-4:** Wire `--related` into the `search` and `hybrid` handlers:
      add the CLI flag, open a `SqliteGraphStore` only when set, enrich each
      result through the helper, and pass the context to the human and JSON
      renderers; ranked results and the non-`--related` output are unchanged.
  - Depends on: TASK-013-3
  - Verification: `cargo test -p kv-app` passes; human and JSON outputs include
      the related context only when `--related` is set.

- [x] **TASK-013-5:** Write failing CLI acceptance tests for `mdsearch context`:
      neighbors returned as JSON, node lookup, the missing-`--collection` error,
      the unknown-node error, the malformed-query error, and the
      missing-database error that must not create a file.
  - Depends on: None (reuses the EPIC-005 in-process schema)
  - Verification: App tests cite `REQ-013`/`FR-006`..`FR-010` and the
      corresponding Gherkin scenarios and fail before the command is wired.

- [x] **TASK-013-6:** Implement the `mdsearch context '<query>'` subcommand: CLI
      arguments (positional query, `--collection` required, `--database`), the
      handler that opens `SqliteGraphStore`, builds the schema with
      `build_schema`/`handle`, executes the query on a current-thread tokio
      runtime, prints the JSON response, and maps typed failures to clear errors.
  - Depends on: TASK-013-5
  - Verification: `cargo test -p kv-app` passes; CLI and schema tests are green,
      the command is read-only, and a missing database does not create a file.

- [x] **TASK-013-7:** Execute the complete acceptance and constitution
      verification pass, then review the implementation against the approved
      packet. Run every scenario in `scenarios.feature`, verify all `REQ-013`
      functional requirements and failure paths, check existing commands
      (`search`, `hybrid`, `embed`, `get`, `add`, `update`, `graph`) for
      regressions, and capture observed gate output.
  - Depends on: TASK-013-4, TASK-013-6
  - Verification: `cargo xtask ci` passes with observed output for formatting,
      clippy with `-D warnings`, workspace tests, docs, `cargo deny`, and
      coverage; all Gherkin scenarios pass; schema links/statuses and ADR/design
      traceability are current.

## Test And Verification Plan

- [x] Unit checks: related-enrichment filter (File destinations, closed relation
      set, deduplication, empty on missing node/graph).
- [x] Rendering checks: human `related:` lines and JSON `related` array shape,
      additive output, unchanged ranked results.
- [x] CLI acceptance checks: `--related` on `search` and `hybrid` (human + JSON)
      and `mdsearch context` neighbors/node/missing-collection/unknown-node/
      malformed-query/missing-database behavior.
- [x] Gherkin scenarios: every scenario in `scenarios.feature`.
- [x] Non-functional checks: read-only behavior (no graph/database/file writes),
      offline with no LLM or network access, deterministic output, no GraphQL
      server or network endpoint.
- [x] Constitution checks: tests-first red-to-green evidence, traceable test
      names/comments, public documentation, dependency/layering rules, and
      `cargo xtask ci` with the Definition of Done checklist.
- [x] Regression checks: `search`, `hybrid`, `embed`, `get`, `add`, `update`,
      `graph`, and collection lifecycle behavior remain unchanged.

## Rollout And Recovery

### Rollout

Ship both surfaces in the single compiled binary. No schema migration is
required: the entity graph is already built by `mdsearch update` (EPIC-005), and
existing databases receive their graph on the next successful update. `--related`
and `mdsearch context` read that graph; the in-process `async_graphql` layer
remains compiled into the app with no server endpoint or new runtime service.

### Recovery

Both surfaces are strictly read-only, so there is nothing to roll back: a
missing graph or node yields an empty related set (or a clear node-not-found
error for `mdsearch context`), and a failed or missing database target reports an
error without creating or modifying the database. If the graph is stale or
absent, the user re-runs `mdsearch update`; no partial state is ever written. A
failed dependency/license gate blocks the build before release and does not alter
runtime data.

## Definition Of Done

- [x] All ordered tasks are complete and their verification checks are observed.
- [x] Unit, rendering, CLI, and acceptance tests pass, including every specified
      error and recovery path.
- [x] Every scenario in `scenarios.feature` passes.
- [x] `cargo xtask ci` passes cleanly with the required coverage threshold.
- [x] Existing commands and output have no regression without `--related`.
- [x] Offline, read-only, no-LLM, no-network, and no-server constraints are
      verified.
- [x] Approved specifications and ADR traceability remain current.
- [x] No graph build changes, LLM/claim extraction, external services, or
      speculative context behavior were added.
