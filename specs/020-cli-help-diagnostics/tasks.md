---
id: TASK-020
title: "CLI help, diagnostics, and global options implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved TASK-020 for implementation in chat on 2026-09-30"
parent: US-020
depends_on: []
requires: []
blockers: []
related:
  - REQ-020
  - DES-020
  - ADR-001
  - ADR-012
---

# Tasks

## Implementation Approach

Implement the approved DES-020 in the existing app crate, using implement-feature
after this task plan is approved and promote-artifact validates spec readiness.
Keep each RED/GREEN pair observable and record its commands and results. Tests
cite REQ-020 functional requirements and cover the sibling scenarios. No new
dependency, workspace member, architectural layer, schema, or port is introduced.

The scope is current-grammar help, diagnostics, global database selection,
collection/limit short switches, and the renamed ingestion switch. Persisted
sources, atomic combined indexing, semantic/model management, unified retrieval,
graph scoping, richer status, exact get output, and the final breaking grammar
remain subsequent slices.

## Ordered Tasks

- [x] **TASK-020-1 (RED):** Add actual subprocess regression tests for information,
  diagnostics, and configuration resolution.
  - Depends on: None; task approval and packet spec readiness are prerequisites.
  - Outcome: `crates/app/tests/cli_diagnostics.rs` covers root/nested help and
    version without HOME, unknown/missing arguments, missing database and home,
    explicit database placement at each depth, and model-cache overrides without
    HOME when the database is explicit (REQ-020 FR-001 through FR-005).
  - Verification: Run `cargo test -p kv-app --test cli_diagnostics` and record
    failures showing the existing wrong exits/streams, early HOME requirement,
    or rejected database placement. Use subprocess environment overrides and
    temporary paths; no process-global environment mutation or model downloads.

- [x] **TASK-020-2 (GREEN):** Implement parse-first invocation, resolved paths,
  global database selection, and process diagnostic handling.
  - Depends on: TASK-020-1.
  - Outcome: Add the DES-020 invocation module and environment entry point,
    preserve the injected-home entry point, pass concrete paths to execution,
    retain ADR-012 cache precedence, add HomeUnavailable, and render clap
    information/argument errors with exit codes 0/2 and operational errors with 1.
  - Verification: Add pure path-resolution tests before implementing their
    corresponding helper behavior; observe failure then success. TASK-020-1
    passes. A lazy home lookup is tested for being unnecessary with explicit
    paths and needed only for defaults. The binary stays within ~50 lines.

- [x] **TASK-020-3 (RED):** Add acceptance tests for descriptive help, short
  switches, search limits, and the ingestion spelling change.
  - Depends on: TASK-020-2.
  - Outcome: `crates/app/tests/cli_options.rs` covers `-c` on every existing
    collection switch, `-n` on search/hybrid, default 10, limits 1/100 and 0/101,
    help descriptions/examples at every depth, skipping unreadable paths under
    the new spelling, and rejection of `--force` (REQ-020 FR-006 through FR-009).
  - Verification: Run `cargo test -p kv-app --test cli_options` and observe the
    missing-short-switch, missing-help, and old-ingestion-spelling failures. Use
    missing file paths for deterministic unreadable-path cases. Update existing
    add/update test invocation spellings and traceability while retaining their
    assertions, then observe the renamed invocations fail before parser changes.
    Hybrid parser acceptance cases need no model downloads or inference.

- [x] **TASK-020-4 (GREEN):** Add descriptive CLI help/examples and implement
  short switches and `--skip-unreadable`.
  - Depends on: TASK-020-3.
  - Outcome: Every root/group/leaf has meaningful purpose/effect help, applicable
    prerequisites/defaults, argument descriptions, and invocation examples.
    Existing switch-based collection selection and search/hybrid limits gain
    short spellings. Ingestion passes the renamed field to unchanged use cases.
  - Verification: TASK-020-3 and the preserved ingestion tests pass. Run
    `cargo test -p kv-app` to check existing CLI and retrieval contracts.

- [x] **TASK-020-5 (REFACTOR / TRACE):** Move existing rendering into the design's
  rendering module, complete bounded dispatch organization, and audit traceability.
  - Depends on: TASK-020-4.
  - Outcome: App modules have clear responsibilities; run.rs falls below the
    700 non-test-line limit and dispatch functions stay within constitutional
    limits. Preserve rendering tests and successful JSON/human contracts. Both
    public entry points have error documentation and compiled examples. Every
    changed behavior/test cites REQ-020.
  - Verification: `cargo test -p kv-app` passes without changing rendering
    assertions. Review module/function sizes, visibility, dependency direction,
    typed error classification, and absence of new production unwrap/unsafe or
    lint suppressions. No port contract changes require additional port suites.

- [x] **TASK-020-6 (DOCUMENTATION):** Correct the README and check examples against
  the current binary (REQ-020 FR-010).
  - Depends on: TASK-020-5.
  - Outcome: Document global database placement, short collection/limit switches,
    information/argument/operational exit codes, lazy defaults, and unreadable
    skipping. Remove forced-reprocessing descriptions and old switch usage from
    the current command reference. Explain this slice's switch replacement;
    leave final command mapping to the later grammar slice.
  - Verification: Inspect root/group/leaf help and README links. Exercise local
    database/search/ingestion examples with temporary fixtures; inspect semantic
    examples for prerequisite/download accuracy without downloading assets.

- [x] **TASK-020-7 (VERIFY):** Use verify-feature to execute quality gates,
  review scenario coverage, and record observed completion evidence.
  - Depends on: TASK-020-6.
  - Outcome: Append dated RED/GREEN, quality, coverage, and retrieval evaluation
    evidence with the tested commit hash and working-tree qualification. Check
    completed tasks and transition lifecycle state only after all required gates
    pass through the mapped skills.
  - Verification: Execute `cargo xtask validate-specs`, `cargo xtask ci`, and
    `cargo xtask eval`; report actual outputs and any failures. Preserve the
    existing evaluation baseline. Verify the Constitution's workspace/domain
    coverage thresholds from observed coverage output. No claim of implementation
    completion if a required gate fails.

## Test And Verification Plan

- [x] REQ-020 FR-001/FR-002: stdout/stderr and 0/2 subprocess exits for help,
  version, unknown options, required arguments, and invalid numeric limits.
- [x] REQ-020 FR-003: stderr/exit 1 for missing database and HOME defaults.
- [x] REQ-020 FR-004/FR-005: global database positions, no unintended default
  database, lazy HOME, explicit cache overrides, and ADR-012 precedence.
- [x] REQ-020 FR-006/FR-007: collection switch spellings, search/hybrid limit
  spellings, default 10, and inclusive boundaries 1–100.
- [x] REQ-020 FR-008: root/group/leaf purpose/effects, prerequisites/defaults,
  argument descriptions, and examples.
- [x] REQ-020 FR-009: renamed skip switch, skipped counts, successful readable
  ingestion, existing atomic-failure behavior, and old-switch rejection.
- [x] REQ-020 FR-010: current README and help match tested behavior.
- [x] Map every scenario in [scenarios.feature](scenarios.feature) to observed
  passing tests or documentation checks; preserve existing retrieval/JSON tests.
- [x] Pure unit checks use injected paths only; subprocess/integration tests own
  temporary filesystem/database fixtures and isolated environment overrides.
- [x] Observe and record `cargo xtask validate-specs`, `cargo xtask ci`, and
  `cargo xtask eval` outputs via verify-feature.
- [x] Review module/function sizes, public docs/examples, unchanged dependencies,
  and Constitution Definition of Done.

### Implementation And Verification Evidence

Implementation was performed test-first on 2026-09-30. RED evidence:
`cargo test -p kv-app --test cli_diagnostics` initially reported 10 failures
among 11 subprocess cases; `cargo test -p kv-app --test cli_options` initially
reported 17 failures among 20 cases. After implementation, the diagnostic
suite passed 11/11 and the options suite passed 30/30. `cargo test -p kv-app
--offline` passed all app unit, integration, and doctests.

The `verify-feature` gates were observed on 2026-09-30 against HEAD
`1eee326` with the working tree containing this feature's changes (not committed):

- `cargo xtask validate-specs`: exit 0; 111 artifacts scanned and all checks
  passed. This was rerun after recording the completion evidence.
- `cargo xtask ci`: exit 0; formatting, Clippy with warnings denied, all
  workspace/all-feature tests, rustdoc warnings, cargo-deny, and coverage passed.
  Coverage was 87.79% line, 87.56% regions, and 92.25% functions, above the
  85% line threshold. The first CI attempt exposed five panic-prone JSON indexing
  assertions, which were changed to checked lookups. The next attempt exposed
  locked `rustls 0.23.43` (RUSTSEC-2026-0285); Cargo updated the compatible lockfile
  entry to 0.23.45, after which advisories passed.
- `cargo xtask eval`: exit 0; 32 queries with Recall@5 1.0000, MRR@5 1.0000,
  and NDCG@5 1.0000; all ADR-004 thresholds passed.
- `cargo xtask eval --verify-only`: exit 0; 32 documents, 32 queries, and 78
  judgments verified.
- `git diff --check`: exit 0. No dependency was added; the lockfile patch only
  advances the existing `rustls` version to address the CI advisory.

RED/GREEN coverage maps to the test modules named in TASK-020-1 and TASK-020-3;
existing ingestion and retrieval/JSON suites also passed in CI. This packet
covers only the first CLI help/diagnostics slice. The later source persistence,
atomic combined indexing, model management, unified retrieval, graph scoping,
status, and breaking command grammar remain separate work.

## Rollout And Recovery

### Rollout

Deliver the first slice in the existing grammar. Existing databases require no
migration; the existing injected-home library API is preserved. CLI consumers
replace `--force` with `--skip-unreadable` and may use global database placement
and short switches. Help/version now succeed, parser errors exit 2, and
operational errors retain exit 1. Do not commit or publish without a user request.

### Recovery

Information and parsing/configuration failures perform no database mutation.
Operational rollback and retry remain governed by the unchanged use cases.
Correct rejected options or missing defaults and rerun; model downloads remain
explicit opt-in. This slice adds no partial-success or retry policy. Reverting
the CLI-only change needs no database downgrade; callers would restore previous
spellings. Preserve user-owned working-tree changes during all edits.

## Definition Of Done

- [x] The complete packet is approved and passes the spec-ready predicate.
- [x] All seven tasks are complete with recorded RED/GREEN evidence.
- [x] Every approved requirement and scenario has passing traceable coverage.
- [x] Existing ingestion, retrieval, graph, and JSON contracts pass.
- [x] Configuration and error handling satisfy DES-020 and the Constitution.
- [x] Public entry points have error docs and compiled examples.
- [x] README/help corrections and local examples are checked.
- [x] Required validation, CI, coverage, and retrieval evaluation gates pass with
  observed evidence recorded through verify-feature.
- [x] No unapproved dependency/layer, weakened assertion, ignored test, unsafe
  code, or lint suppression was introduced.
- [x] Lifecycle completion is recorded through the mapped skills only after
  verification; later redesign slices remain separate work.
