---
id: TASK-017
title: "Model downloads live under .mdsearch with reliable availability detection implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-017
related:
  - REQ-017
  - DES-017
  - ADR-012
  - REQ-010
  - REQ-011
  - US-010
  - US-011
---

# Tasks

## Implementation Approach

Implement the approved changes from `DES-017`/`ADR-012` in dependency order:
tests first and observed red (R-TST-01), then the adapter production change
(marker-based availability, required cache directory), then the `app`
resolution and wiring, then documentation, then the constitution gates.

The production surface is confined to `crates/adapters/embed-fastembed` and
`crates/app`:

- Cache resolution moves to `app`: `HF_HOME` → `FASTEMBED_CACHE_DIR` →
  `~/.mdsearch/models`; the working-directory `.fastembed_cache` fallback is
  deleted from the adapters.
- `FastembedGenerator::new` / `FastembedReranker::new` take a required
  `PathBuf`; production call sites are `crates/app/src/run.rs:310-311` (hybrid)
  and `crates/app/src/run.rs:693-694` (embed).
- Availability = per-model completion marker `{cache_dir}/{model}.completed`
  written atomically after a successful session build; the hf-hub layout probe
  (`model_is_cached`) is deleted from both adapters.
- Port signatures, error variants, and the "pass --download" message text are
  unchanged (REQ-017 FR-009).

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit
(R-AGT-01), write tests before implementation and observe them red (R-TST-01),
and do not add dependencies, workspace members, layers, unsafe code, or lint
suppressions without the required approval (R-AGT-02, R-AGT-04, R-TOOL-03).

This slice introduces no new dependency, no port/use-case/domain change, no
CLI switch change, and no legacy-cache migration. The real-download happy path
is not exercised in automated tests (no network, R-TST-05); it stays covered
by the existing REQ-010 download-gating contract tests.

## Ordered Tasks

- [ ] **TASK-001:** Adapter red tests in `embed-fastembed` — marker helpers
  (`marker_path`, `marker_exists`, `write_marker` atomic round-trip in a temp
  dir) and availability short-circuit: `ensure_available(model, false)` returns
  `Ok` when the marker exists (no session build, no network) and
  `ModelNotCached` when absent, for both the generator and the reranker.
  - Depends on: approved feature packet (US-017, REQ-017, DES-017, ADR-012)
  - Verification: new tests fail (red) before production code exists
- [ ] **TASK-002:** `app` red tests — `model_cache_dir` resolution order:
  `HF_HOME` wins over `FASTEMBED_CACHE_DIR`; either env wins over the home
  default; no env → `home/.mdsearch/models`.
  - Depends on: TASK-001
  - Verification: new tests fail (red) before the helper exists
- [ ] **TASK-003:** Adapter production change in `embed-fastembed` — required
  `cache_dir: PathBuf` constructors; shared marker helpers; `model_is_cached`
  layout probe deleted and replaced by marker checks in `ensure_available` and
  the lazy session builds of `embed()`/`rerank()`; cache dir created on demand
  in the download path; marker written atomically after session build, with
  marker-write failure failing the download cleanly (REQ-010 FR-010
  semantics). Update the adapter's own tests' constructor calls.
  - Depends on: TASK-001
  - Verification: TASK-001 tests green; `cargo test -p embed-fastembed`
- [ ] **TASK-004:** `app` production change — `model_cache_dir` helper; pass
  the resolved directory to both adapters in `embed()` (run.rs:693-694) and
  `hybrid()` (run.rs:310-311); update the `--download` argument help in
  `crates/app/src/cli.rs` to state the default location and resolution order.
  - Depends on: TASK-002, TASK-003
  - Verification: TASK-002 tests green; existing `crates/app/tests/embed.rs`
    and `crates/app/tests/hybrid_search.rs` pass unchanged (error-path
    behavior preserved)
- [ ] **TASK-005:** Documentation — README section on model assets: resolution
  order (`HF_HOME` → `FASTEMBED_CACHE_DIR` → `~/.mdsearch/models`), the
  completion-marker contract, and the one-time re-download for legacy
  `.fastembed_cache` users.
  - Depends on: TASK-004
  - Verification: paths and links verified; no stale references
- [ ] **TASK-006:** Full verification — trace the offline-reachable scenarios
  of `scenarios.feature` (resolution order, env override, marker presence and
  absence, reranker parity, `--database` independence) to the new and existing
  tests; run the constitution gates (R-TOOL-04) and observe output.
  - Depends on: TASK-003, TASK-004, TASK-005
  - Verification: `cargo fmt --all -- --check`, `cargo clippy --workspace
    --all-targets --all-features -- -D warnings`, `cargo test --workspace
    --all-features`, `cargo doc --workspace --no-deps`
- [ ] **TASK-007:** Specification statuses — mark `requirements.md`,
  `design.md`, and `tasks.md` approved/implemented, `ADR-012` accepted,
  story/REQ/DES/TASK IDs and PRD-001 (EPIC-011, DEC-016) and TODO.md
  (OBS-014/OBS-015 PROMOTED) references consistent.
  - Depends on: TASK-006
  - Verification: traceability audit across the packet, PRD, TODO, and ADR

## Test And Verification Plan

- [ ] Unit or component checks: marker helpers and availability short-circuit
  in `embed-fastembed`; `model_cache_dir` resolution in `app` (red first)
- [ ] Gherkin scenarios: `scenarios.feature` — offline-reachable scenarios
  traced to adapter and app tests; download-path scenarios covered by the
  existing REQ-010 contract tests and manual verification
- [ ] CLI checks: `crates/app/tests/embed.rs` and `hybrid_search.rs` error
  paths unchanged; `--download` help text updated
- [ ] Non-functional checks: no network in tests; availability is a single
  `Path::exists`; offline default preserved; no new dependencies

## Rollout And Recovery

### Rollout

Additive behavior change to the default cache location. Users with an existing
cache under a legacy `.fastembed_cache` or an environment location are
unaffected while the env overrides remain set; users relying on the old CWD
default re-download once. No database schema change, no migration, no CLI
surface change.

### Recovery

- A failed or interrupted download leaves no marker: the next run truthfully
  advises `--download` and retries safely (REQ-010 FR-010 semantics).
- Deleting a marker file re-arms the download advice; deleting a marker does
  not delete model assets.
- Marker present but assets deleted externally: a storage error at session
  build; re-running `--download` rewrites the assets and the marker.

## Definition Of Done

- [ ] All tasks are complete, with red-then-green observed on the new tests.
- [ ] `cargo xtask ci` gates pass with observed output (R-TOOL-04).
- [ ] The executable scenarios are traced; offline-reachable scenarios are
  covered by tests.
- [ ] Relevant specifications are updated and statuses consistent
  (R-SDD-05).
- [ ] README and CLI help document the resolution order and marker contract.