---
id: TASK-019
title: "Wikilink graph extraction implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-019
related:
  - REQ-019
  - DES-019
  - ADR-014
  - REQ-012
  - US-012
---

# Tasks

## Implementation Approach

Implement the approved changes from `DES-019`/`ADR-014` in dependency order:
tests first and observed red (R-TST-01), then the domain extraction and
resolution functions, then the `extract_graph` integration, then integration
coverage and the constitution gates.

The production surface is confined to `crates/domain/src/graph.rs`:

- `inline_wikilinks(content)` extracts `[[target]]`, `[[target|label]]`,
  `[[path/target#heading]]`, and `[[target#heading|label]]` path targets,
  dropping labels, stripping fragments, and ignoring `[[#...]]`, `[[]]`, and
  `http(s)` targets.
- `resolve_wikilink(target, source_path, known)` mirrors `resolve_file`'s
  candidate strategy with case-insensitive known-path matching; candidates
  with more than one case-insensitive match are ambiguous and skipped.
- `extract_graph` gains a wikilink pass after the markdown-link pass,
  skipping resolved self-edges. `resolve_file` and markdown-link behavior are
  untouched.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces no dependency, no schema change, and no store, adapter,
or CLI changes. Code-fence awareness (OBS-010) is out of scope.

## Ordered Tasks

- [ ] **TASK-001:** Red domain unit tests in `crates/domain/src/graph.rs` —
  `inline_wikilinks` forms and stripping; `resolve_wikilink` case-insensitive
  resolution, ambiguity skipping, unresolved `None`; `extract_graph` wikilink
  edges including self-edge skip and markdown-link coexistence.
  - Depends on: approved feature packet (US-019, REQ-019, DES-019, ADR-014)
  - Verification: new tests fail (red) before the production functions exist
- [ ] **TASK-002:** Domain production change — implement `inline_wikilinks`
  and `resolve_wikilink` in `crates/domain/src/graph.rs`; add the wikilink
  pass to `extract_graph` with self-edge skipping.
  - Depends on: TASK-001
  - Verification: TASK-001 tests green; `cargo test -p kv-domain` passes
- [ ] **TASK-003:** Integration coverage — the `012-entity-graph` scenarios
  (already extended) map to store/app graph-rebuild paths: exercise graph
  rebuild from files containing wikilinks (plain, piped, fragment, case
  variant, unresolved, self) through the existing store integration or CLI
  tests; confirm existing graph tests are unchanged.
  - Depends on: TASK-002
  - Verification: workspace tests green; graph-rebuild behavior verified
- [ ] **TASK-004:** Full verification — trace the `scenarios.feature`
  scenarios to the new tests; run the constitution gates (R-TOOL-04) and
  observe output.
  - Depends on: TASK-002, TASK-003
  - Verification: `cargo xtask ci` passes with observed output
- [ ] **TASK-005:** Specification statuses — mark `requirements.md`,
  `design.md`, and `tasks.md` approved/implemented, `ADR-014` approved;
  verify PRD-001 (EPIC-013, DEC-018), TODO.md (OBS-009 PROMOTED), and the
  `012-entity-graph` scenario revision are consistent.
  - Depends on: TASK-004
  - Verification: traceability audit across the packets, PRD, TODO, and ADR

## Test And Verification Plan

- [ ] Unit or component checks: domain unit tests for forms, stripping,
  case-insensitivity, ambiguity, unresolved, self-skip, coexistence, and
  determinism
- [ ] Gherkin scenarios: `019` packet scenarios + the `012-entity-graph`
  revision (wikilink LINKS_TO, fragment/label, case/ambiguity, unresolved and
  self-skip)
- [ ] CLI / integration checks: existing graph tests pass unchanged; graph
  rebuild with wikilink files exercised
- [ ] Non-functional checks: domain purity preserved (no I/O); no schema or
  dependency changes; deterministic extraction

## Rollout And Recovery

### Rollout

Additive domain behavior: graphs gain `LINKS_TO` edges for wikilinks on the
next `update`/rebuild. No schema change, no migration, no CLI surface change,
no dependency change. Existing databases regenerate edges on the next update.

### Recovery

- A faulty or ambiguous wikilink is skipped, never fatal: extraction errors
  cannot occur by construction (total functions).
- Re-running `update` is idempotent: the rebuild regenerates the same edge
  set; deleted targets drop edges.

## Definition Of Done

- [ ] All tasks are complete, with red-then-green observed on the new tests.
- [ ] `cargo xtask ci` gates pass with observed output (R-TOOL-04).
- [ ] The executable scenarios are traced; offline-reachable scenarios are
  covered by tests.
- [ ] Relevant specifications are updated and statuses consistent
  (R-SDD-05).
- [ ] Standard markdown-link extraction and all other graph behavior are
  unchanged.