---
id: TASK-016
title: "Destroy a collection completely with no orphaned data implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-016
related:
  - REQ-016
  - DES-016
  - US-003
  - REQ-003
  - ADR-011
---

# Tasks

## Implementation Approach

Implement the approved change from `DES-016`/`ADR-011`: replace the single
`DELETE FROM collections` in `SqliteCollectionStore::destroy_collection`
(`crates/adapters/store-sqlite/src/lib.rs:314`) with an explicit
transactional multi-table delete, in dependency order: first revise `REQ-003`
and the `003-destroy-collection` packet, then drive the store regression tests
red-to-green, then the CLI acceptance surface, then the constitution gates.

The production change is confined to one adapter method. The delete order is
fixed to keep the FTS5 virtual table consistent:

1. `embeddings` (guarded: the table may not exist on databases created after
   the dimension-aware change),
2. `passages` by rowid subset (`WHERE rowid IN (SELECT passage_rowid FROM
   passage_files WHERE collection_id = ?)`), then `passage_files`,
3. `files`, `edges`, `nodes`, `graph_state`, `lexical_index_state`,
   `semantic_index_state`,
4. `collections` with `RETURNING display_name` for the confirmation output.

The whole sequence runs in one transaction; any failure rolls back, leaving
the collection and its data intact. `settings` and other collections are never
touched.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces no new dependency, migration, schema change, or CLI
change.

## Ordered Tasks

- [x] **TASK-016-1:** Revise `REQ-003` (`specs/003-destroy-collection/requirements.md`)
      so its destroy contract states that a successful destroy removes the
      collection and all of its data (files, passages, vectors, graph, index
      state), and add regression scenarios to the `003-destroy-collection`
      packet (destroy leaves no trace; destroy-then-recreate surfaces no stale
      data; a failed destroy leaves the collection intact).
  - Depends on: None
  - Verification: the revised packet contains no claim that destroy leaves
    data behind; scenarios and requirements agree with `REQ-016`/`DES-016`
    (R-SDD-05).

- [x] **TASK-016-2 (RED):** Extend the store integration tests
      (`crates/adapters/store-sqlite/tests/destroy_collection.rs`) with: (a)
      SQL-level emptiness of all ten per-collection tables after destroying a
      fully indexed collection (files, lexical, semantic, graph); (b) a
      destroy of a files-only collection leaves no `files` rows; (c) other
      collections remain fully intact; (d) atomicity: an injected
      `RAISE(ABORT)` trigger on one child table fails the destroy and leaves
      the collection and every table unchanged.
  - Depends on: TASK-016-1
  - Verification: the emptiness and atomicity tests fail red against the
    current single-statement destroy (observed failing output).

- [x] **TASK-016-3 (GREEN):** Implement the transactional multi-table delete in
      `SqliteCollectionStore::destroy_collection` per `DES-016`: resolve the
      `collection_id`, delete `embeddings` (guarded by a table-existence
      check), `passages` by rowid subset, `passage_files`, `files`, `edges`,
      `nodes`, `graph_state`, `lexical_index_state`, `semantic_index_state`,
      then `collections` with `RETURNING display_name`, all in one committed
      transaction.
  - Depends on: TASK-016-2
  - Verification: TASK-016-2 tests pass green; the full `kv-store-sqlite`
      suite still passes.

- [x] **TASK-016-4:** Add CLI acceptance coverage mapped from
      `scenarios.feature` for the offline-reachable paths: destroying a fully
      indexed collection, destroying one of two collections, the
      destroy-then-recreate scenario (no stale results from search, hybrid,
      or graph), and the failed-destroy atomicity path at the store level.
  - Depends on: TASK-016-3
  - Verification: the 016 scenarios and the revised `003-destroy-collection`
      scenarios pass as executable acceptance tests; existing destroy CLI
      tests remain green.

- [x] **TASK-016-5:** Execute the constitution gates and Definition of Done:
      `cargo xtask ci` with observed output; confirm `US-016`, `REQ-016`,
      `DES-016`, `ADR-011`, `REQ-003`, and the 003 packet are mutually
      consistent.
  - Depends on: TASK-016-4
  - Verification: observed command output from every gate, per R-AGT-07; no
    new warnings, suppressions, or unapproved artifacts.

## Test And Verification Plan

- [x] Integration checks: SQL-level emptiness of all ten per-collection tables
      after destroy; files-only destroy; other-collection isolation; atomicity
      on injected failure (TASK-016-2).
- [x] CLI checks: destroy confirmation, destroy-then-recreate surfaces no
      stale results, existing destroy behavior unchanged (TASK-016-4).
- [x] Gherkin scenarios: `scenarios.feature` (016) and the revised
      `003-destroy-collection` packet.
- [x] Non-functional checks: no new dependencies, migrations, schema changes,
      or CLI changes; offline behavior unchanged; no VACUUM or file deletion.
- [x] Constitution checks: `cargo xtask ci` and the Definition of Done gates
      with observed output (TASK-016-5).

## Rollout And Recovery

### Rollout

Ship the change in the single compiled binary. No migration and no schema
change are required; the behavior change affects only the destroy path, which
now removes all of the collection's data instead of leaving orphans. Existing
databases with pre-existing orphaned rows are not repaired by this slice;
a re-destroy of the affected collection (or a re-created collection) benefits
from the new cleanup only going forward.

### Recovery

A failed destroy rolls back atomically, leaving the collection and its data
intact; re-running the command retries safely. Destroy remains irreversible on
success — now completely so, which is the intent. The database file is never
deleted and no space is reclaimed, so no storage-layer recovery is involved.

## Definition Of Done

- [x] All tasks are complete.
- [x] Tests were written first and observed to fail (TASK-016-2).
- [x] Every behavior in `REQ-016` is covered and traceable to a test
      (R-SDD-02).
- [x] The executable scenarios for 016 and the revised 003 packet pass.
- [x] The Rust constitution's tooling gates pass with observed output
      (R-TOOL-04, R-AGT-07).
- [x] `US-016`, `REQ-016`, `DES-016`, `ADR-011`, `REQ-003`, and the 003 packet
      are consistent (R-SDD-05).
- [x] No new dependencies, workspace members, layers, unsafe code, or lint
      suppressions were introduced.
- [x] No out-of-scope behavior (FK PRAGMA, VACUUM, file deletion, CLI changes)
      was added.