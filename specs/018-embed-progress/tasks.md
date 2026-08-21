---
id: TASK-018
title: "embed shows live ingestion progress on stderr implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-018
related:
  - REQ-018
  - DES-018
  - ADR-013
  - REQ-010
  - US-010
  - US-017
---

# Tasks

## Implementation Approach

Implement the approved changes from `DES-018`/`ADR-013` in dependency order:
tests first and observed red (R-TST-01), then the application-layer progress
events, then the `indicatif` dependency and the stderr renderer in `app`, then
the constitution gates.

The production surface is confined to `crates/application` (embed use case)
and `crates/app` (renderer and wiring):

- `EmbedProgress` (`Files` per file, `Writing` before rebuild) emitted by
  `EmbedCollections::execute` through a `&mut dyn FnMut(EmbedProgress)`
  callback; the generator port is unchanged; per-file embedding preserves
  passage order (store query `ORDER BY file_id, position`).
- `indicatif = "0.18"` added to `[workspace.dependencies]` and `kv-app`
  (owner-approved dependency, ADR-013).
- `ProgressRenderer` in `app` draws a per-collection bar to stderr and a
  single status line for the write phase; stdout and the embed report are
  unchanged.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces one approved dependency (`indicatif`), no store/port
changes, no CLI switch changes, and no progress on other commands or phases.

## Ordered Tasks

- [ ] **TASK-001:** Application red tests in
  `crates/application/tests/embed_collections.rs` — a recording progress
  callback plus the existing fakes assert: per-file `Files` events with
  correct `completed_files`/`total_files` for a multi-file collection; a
  `Writing` event before the rebuild; no events for skipped (no files, no
  lexical index) and already-current collections; a mid-run generator failure
  stops events and yields the `Failed` outcome.
  - Depends on: approved feature packet (US-018, REQ-018, DES-018, ADR-013)
  - Verification: new tests fail (red) before the event emission exists
- [ ] **TASK-002:** Application production change — `EmbedProgress` enum;
  `EmbedCollections::execute` gains the progress callback parameter; group the
  store-ordered passages by file, embed per file (appending pairs in order),
  emit `Files` after each file and `Writing` before `store.rebuild`; skipped
  and already-current paths emit nothing. Update the existing
  `execute` call sites in tests with a no-op callback.
  - Depends on: TASK-001
  - Verification: TASK-001 tests green; existing `crates/application` tests
    pass unchanged
- [ ] **TASK-003:** Dependency and renderer — add `indicatif = "0.18"` to
  `[workspace.dependencies]` and `kv-app`; add `crates/app/src/progress.rs`
  with `ProgressRenderer` (per-collection `ProgressBar` drawn to stderr via
  `ProgressDrawTarget::stderr()`, message `embedding {collection}`, position
  = completed files; `Writing` finalizes the bar and writes
  `writing semantic index for {collection}...` to stderr via
  `writeln!(io::stderr().lock())`); wire into `run::embed`, finalizing any
  in-flight bar when `execute` returns.
  - Depends on: TASK-002
  - Verification: `cargo test -p kv-app` green; `cargo clippy` clean;
    manual run shows the bar on stderr and the status line, stdout unchanged
- [ ] **TASK-004:** Full verification — trace the `scenarios.feature`
  offline-reachable scenarios to the new tests; run the constitution gates
  (R-TOOL-04) and observe output.
  - Depends on: TASK-002, TASK-003
  - Verification: `cargo xtask ci` passes with observed output
- [ ] **TASK-005:** Specification statuses — mark `requirements.md`,
  `design.md`, and `tasks.md` approved/implemented, `ADR-013` approved; verify
  PRD-001 (EPIC-012, DEC-017) and TODO.md (OBS-016 PROMOTED) references are
  consistent.
  - Depends on: TASK-004
  - Verification: traceability audit across the packet, PRD, TODO, and ADR

## Test And Verification Plan

- [ ] Unit or component checks: application-level progress events with fakes
  (per-file counts, `Writing` ordering, silence for skipped/current, failure
  stop)
- [ ] Gherkin scenarios: `scenarios.feature` — offline-reachable scenarios
  traced to the application tests; stderr rendering verified manually (the
  in-process CLI harness cannot capture stderr)
- [ ] CLI checks: existing `crates/app/tests/*` pass unchanged (stdout
  contract)
- [ ] Non-functional checks: no stdout changes; no new latency beyond one
  event per file; progress best-effort (render failure never fails the run);
  one approved new dependency

## Rollout And Recovery

### Rollout

Additive UX change: stderr gains progress; stdout output is byte-identical.
No database schema change, no migration, no CLI switch change, no behavior
change for harness callers. The `indicatif` dependency is added to the
workspace manifest.

### Recovery

- A failed or interrupted run leaves no half-drawn progress state: the
  renderer finalizes any in-flight bar when `execute` returns, and the
  per-collection failure outcome is reported exactly as before.
- Progress rendering failures (e.g. closed stderr) are best-effort: the run
  outcome is unaffected.

## Definition Of Done

- [ ] All tasks are complete, with red-then-green observed on the new tests.
- [ ] `cargo xtask ci` gates pass with observed output (R-TOOL-04).
- [ ] The executable scenarios are traced; offline-reachable scenarios are
  covered by tests.
- [ ] Relevant specifications are updated and statuses consistent
  (R-SDD-05).
- [ ] The stdout embed report is unchanged; progress appears only on stderr.