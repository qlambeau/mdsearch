---
id: TASK-023
title: "Unified retrieval and final CLI implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the TASK-023 sequence in chat"
parent: US-023
depends_on:
  - US-008
  - US-009
  - US-011
  - US-013
  - US-020
  - US-021
  - US-022
requires:
  - ADR-004
  - ADR-008
  - ADR-009
  - ADR-010
  - ADR-016
  - ADR-017
  - ADR-018
  - ADR-019
blockers: []
related:
  - REQ-023
  - DES-023
  - PRD-001
---

# Tasks

## Implementation Approach

Implement the approved final CLI within existing layers and dependencies.
Observe RED before production changes in each task, then GREEN and refactor
with requirement/scenario traceability. Preserve ranking, JSON fields,
transactional updates, and model behavior. No schema migration is required.

## Ordered Tasks

- [x] **TASK-023-1:** Verify the baseline and complete missing US-011
  verification evidence via verify-feature; validate packet readiness via
  promote-artifact before changing Rust.
  - Depends on: Approved packet and implemented dependency capabilities.
  - Verification: Baseline CI, evaluation, and spec validation observed; record
    US-011 evidence and resolve its incomplete verification lifecycle.
- [x] **TASK-023-2 (RED -> GREEN):** Replace command grammar, enforce argument
  selectors/ranges/modes, introduce typed report/content output, explicit get
  selectors, and resolve HOME/default paths only when needed.
  - Depends on: TASK-023-1.
  - Verification: Subprocess help/version/argument streams and exit codes,
    obsolete commands, global option placement, exact binary/empty/newline
    retrieval, numeric filenames, and lexical updates without HOME pass.
- [x] **TASK-023-3 (RED -> GREEN):** Implement complete structured update
  reports and collection listing of sources/enabled indexes.
  - Depends on: TASK-023-2.
  - Verification: JSON success stdout and complete failure stderr; every
    attempted collection reported, failed state preserved, other updates
    committed; human and JSON listing preserve name/source fields.
- [x] **TASK-023-4 (RED -> GREEN):** Unify search dispatch and rendering,
  preserve lexical/hybrid ranking and related output, add file IDs/mode,
  explicit no-match output, and final-grammar recovery guidance.
  - Depends on: TASK-023-3.
  - Verification: Quoted/multiple words equivalent; limits and no-rerank
    validation; mode-specific JSON compatibility, stable identities, scores,
    lexical-only hybrid contributions, prerequisites and offline behavior.
- [x] **TASK-023-5 (RED -> GREEN):** Implement explicit neighbor options and
  context-bound GraphQL with no public collection arguments.
  - Depends on: TASK-023-4.
  - Verification: Default file/one-hop behavior, explicit kind/relation/depth,
    bounds, required scope, same-key collections, aliases/multiple roots, and
    validation of attempted collection overrides.
- [x] **TASK-023-6 (RED -> GREEN):** Implement read-only scoped index status,
  model/build metadata, legacy-aware reads, and snapshot-based stored-content
  freshness according to ADR-018.
  - Depends on: TASK-023-5.
  - Verification: Current/stale/unbuilt/disabled indexes, equal-count lexical
    changes, missing/extra mappings, graph/semantic fingerprints, dimensions,
    successful-build retention, legacy schemas, inaccessible/edited sources,
    missing database, and no persistent mutations.
- [x] **TASK-023-7:** Complete command help/examples and README old-to-new
  mapping, migrate existing tests to approved new contracts, refactor touched
  oversized files, and map every scenario to tests.
  - Depends on: TASK-023-6.
  - Verification: Help/manual describe final grammar/defaults/prerequisites;
    unchanged-contract assertions retained; architectural and size review
    passes; all named scenarios have REQ-023 traceability.
- [x] **TASK-023-8:** Execute verification via verify-feature and record
  observed output, date, commit/worktree state, coverage, and eval metrics.
  - Depends on: TASK-023-7.
  - Verification: cargo xtask validate-specs, cargo xtask ci,
    cargo xtask eval --verify-only, cargo xtask eval, and git diff --check pass.

## Test And Verification Plan

- [x] Unit/application checks use pure values and in-memory port fakes for
  selectors, output policy, status values, and existing retrieval algorithms.
- [x] Port contract/integration checks cover every touched implementation,
  identity propagation, legacy reads, index freshness, graph scope, and errors.
- [x] CLI subprocess checks assert actual bytes, streams, statuses, argument
  validation, HOME independence, JSON updates, and read-only operation.
- [x] Every approved scenario in scenarios.feature is mapped to passing tests.
- [x] Existing atomic rollback/model/dimension and retrieval regressions pass.
- [x] Required CI and domain evaluation gates pass; ADR-004 thresholds remain
  Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75.
- [x] No dependency, workspace member, unsafe code, lint relaxation, source
  scan during status, or implicit model download is introduced.

## Rollout And Recovery

### Rollout

Deliver the breaking grammar and documentation together. Preserve schema v9
and legacy read compatibility. Existing collections remain searchable;
collections without registered sources require explicit configuration before
update. Existing migration behavior remains on authorized mutation paths.

### Recovery

Correct invalid arguments using help/mapping; restore missing prerequisites
using the diagnostic's explicit configure/update/model command. Failed
collections retain their previous state and can be retried individually;
update-all preserves successful collections and reports every failure. Global
model-change failures retain prior settings and semantic indexes. Read errors
do not create/migrate a database or mutate content. No commit is made without
an explicit user request.

## Definition Of Done

- [x] All tasks have observed RED/GREEN and completion evidence.
- [x] All scenarios and error/boundary contracts are covered and traced.
- [x] Required tooling/coverage/evaluation gates pass with recorded evidence.
- [x] Architectural boundaries, typed errors/identifiers, and public docs hold.
- [x] Documentation and existing regression tests reflect approved changes.
- [x] Only verified completion is promoted to implemented via verify-feature.

## Observed Evidence

TASK-023-1 (2026-09-30, base `546b923`): baseline `cargo xtask ci` passed
with the line coverage gate (>= 85%) passing and 86.80% region coverage; both evaluation commands passed, all three metrics
1.0000. Specification validation and whitespace checks passed. Dependency
capabilities have implemented task packets; US-011 evidence was completed via
verify-feature. Promote-artifact readiness checklist passed: packet and ADRs
approved, dependencies verified, cross-references valid, no blockers.

## Scenario Traceability

Paths below are relative to the repository. `final_cli` means
`crates/app/tests/final_cli.rs`; `inspection` means
`crates/adapters/store-sqlite/tests/index_inspection.rs`. Existing application
and adapter regression suites retain their ranking and transaction assertions.

| Approved scenario | Requirement | Test evidence |
| --- | --- | --- |
| The normal workflow uses the final command grammar | FR-001/005/016 | final_cli: lexical_workflow_runs_without_home, search_modes_preserve_contracts_and_identity, get_preserves_every_byte |
| Collection creation requires at least one source | FR-002 | final_cli: invalid_selectors_exit_two |
| Collection configuration does not index content | FR-002 | app collection_sources: configure_replaces_sources_and_json_list_reports_canonical_paths; semantic_configuration: configure_accepts_semantic_policy_without_indexing |
| Collection listing reports sources and enabled indexes | FR-003 | final_cli: collection_list_exposes_enabled_indexes; app collection_sources/list_collections suites |
| Collection deletion retains original files | FR-004 | final_cli: collection_delete_retains_original_source_bytes; app destroy_collection suite; SQLite collection deletion/rollback contracts |
| Obsolete commands are rejected | FR-001 | final_cli: obsolete_commands_exit_two (all seven cases) |
| Update requires exactly one scope selector | FR-005 | final_cli: invalid_selectors_exit_two |
| Structured updates retain atomic per-collection outcomes | FR-006/007/027 | final_cli: update_all_json_reports_success_and_failure_on_stderr; existing source/semantic atomic update suites |
| Quoted and multiple-word queries are equivalent | FR-008 | final_cli: search_modes_preserve_contracts_and_identity |
| Search defaults to lexical mode | FR-009 | final_cli: search_modes_preserve_contracts_and_identity; existing lexical ranking contracts |
| Hybrid mode preserves existing retrieval behavior | FR-009/027 | final_cli: hybrid_mode_preserves_contract_and_identity; application hybrid search/fusion tests and domain eval |
| Search retains related-file enrichment | FR-010 | final_cli: related_preserves_ranked_results_in_both_modes; app related suite and renderer unit tests |
| Disabling re-ranking requires hybrid mode | FR-011 | final_cli: invalid_selectors_exit_two |
| Hybrid prerequisite failures include a recovery command | FR-012 | final_cli: hybrid_prerequisite_recovery_uses_final_grammar; recovery unit tests; existing app hybrid prerequisite tests |
| Search limits retain their bounds | FR-013 | app cli_options: search_limit_boundaries_are_enforced; hybrid_search: hybrid_rejects_an_out_of_range_limit |
| Search uses the default limit of ten | FR-013 | app cli_options: search_uses_default_limit_of_ten |
| Human search results identify collection and file | FR-014 | final_cli: search_modes_preserve_contracts_and_identity; human renderer tests |
| Search JSON extends existing contracts | FR-015 | final_cli: search_modes_preserve_contracts_and_identity, hybrid_mode_preserves_contract_and_identity; existing mode-specific JSON assertions |
| Empty searches produce explicit output | FR-014/015 | final_cli: empty_search_is_explicit; app lexical_search/hybrid_search empty human/JSON tests |
| Retrieval emits exact stored content | FR-017 | final_cli: get_preserves_every_byte (empty, binary, final newline, no final newline); output unit test |
| Numeric names remain name selectors | FR-016 | final_cli: numeric_names_remain_names |
| Retrieval rejects conflicting selectors | FR-016 | final_cli: invalid_selectors_exit_two |
| Retrieval preserves scoped not-found and ambiguity errors | FR-018 | app/application get_file suites, SQLite retrieval contracts |
| Neighbor inspection defaults to file nodes and one hop | FR-019 | final_cli: graph_queries_and_neighbors_are_scoped |
| Explicit neighbor options restrict traversal | FR-019 | final_cli: graph_queries_and_neighbors_are_scoped, explicit_graph_kinds_and_depth_bound |
| Neighbor depth retains its bounds | FR-020 | final_cli: invalid_selectors_exit_two, explicit_graph_kinds_and_depth_bound |
| Graph commands require collection selection | FR-019/021 | final_cli: invalid_selectors_exit_two |
| Graph queries are bound to the selected collection | FR-021 | final_cli: graph_queries_and_neighbors_are_scoped (same key, aliases, multiple roots) |
| GraphQL cannot override CLI collection selection | FR-021 | final_cli: graph_scope_override_is_rejected (both public fields) |
| Status reports readiness, enablement, models, and successful builds | FR-022 | final_cli: status_reports_unbuilt_and_disabled_indexes; inspection: semantic_inspection_distinguishes_policy_freshness_and_model_compatibility; readiness unit tests |
| Status can be restricted to a collection | FR-022 | final_cli: status_scope_selects_one_of_multiple_collections; inspection missing-scope contracts |
| Freshness compares indexes with stored content | FR-023 | final_cli: status_compares_stored_bytes_without_scanning_sources; inspection same-count/mapping/semantic tests and legacy_stale_index_is_not_certified_by_inspection |
| Pending filesystem changes do not affect status freshness | FR-023 | final_cli: status_compares_stored_bytes_without_scanning_sources (registered source edited and moved away) |
| Global options work throughout the command tree | FR-024 | app cli_diagnostics: global_database_option_selects_explicit_path; cli_options short-switch tests |
| Diagnostics and informational commands retain their stream contract | FR-025 | app cli_diagnostics subprocess suite; final_cli argument/update failures |
| Explicit paths permit operations without unnecessary HOME resolution | FR-024 | final_cli: lexical_workflow_runs_without_home, lexical_only_hybrid_without_reranking_does_not_resolve_home; cli_options explicit cache tests |
| Help and migration documentation describe the replacement surface | FR-026 | app cli_options: help_describes_command_and_shows_example, readme_documents_current_diagnostic_and_ingestion_contract; README command reference and migration table |


## Final Verification Evidence

Executed through verify-feature on 2026-09-30 against the completed working
tree based on `546b923750bf566e2b003d48307bee9f0e90cbe6`. This slice has not
been committed. No dependencies, workspace members, database schema changes,
unsafe code, or lint relaxations were introduced.

- `cargo xtask ci`: passed. Formatting, warnings-denied clippy, all-feature
  workspace tests, warnings-denied rustdoc, cargo-deny, coverage, and spec
  validation all succeeded. The CI coverage pass also reran the workspace
  tests with instrumentation.
- Coverage: **91.38% lines** (8,392 covered of 9,184), **86.91% regions**,
  **87.69% functions**. Required line threshold: 85%.
- Final subprocess suite: **45 passed**, including actual exits/streams,
  binary and empty retrieval, structured partial failures, graph scope,
  stored-content status, absent sources, and lazy model-path resolution.
- SQLite inspection suite: **19 passed**, including schema versions 0–8,
  stale legacy text, equal-count changes, malformed mappings, semantic
  policy/model compatibility, and non-mutating read errors.
- Existing application/SQLite/CLI ranking, provenance, source discovery,
  deletion, per-collection rollback, global model rebuild/dimension rollback,
  and offline asset regressions passed without weakening their unchanged
  contracts. Fixture ingestion uses application ports to preserve historical
  stored-but-unindexed states; removed commands are tested only as errors.
- `cargo xtask eval --verify-only`: passed (32 documents, 32 queries,
  78 judgments).
- `cargo xtask eval`: passed; Recall@5 **1.0000**, MRR@5 **1.0000**,
  NDCG@5 **1.0000** overall and in every modality; ADR-004 gates passed.
- `cargo xtask validate-specs`: passed; 128 specification artifacts scanned.
- `git diff --check`: passed.

Behavioral tests were authored before their implementation in the TASK-023
RED/GREEN cycles. Prerequisite recovery was observed failing before adding
final-grammar guidance. A final HOME-independence test reproduced exit 1 with
`home directory is unavailable` for lexical-only hybrid retrieval with
`--no-rerank`; after deferring model collaborator path failures until actual
asset use, it passed. Every approved scenario is mapped above. Refactoring
kept transaction ownership and SQL operation order, split the oversized
SQLite composition file into existing adapter responsibilities, and kept
app dispatch under 700 non-test lines with functions within the required
body-size limit. The README contains the final reference and complete
old-to-new command mapping.

The first sandboxed CI run reached cargo-deny but could not obtain its
advisory database lock outside the workspace. The required CI command was
rerun with approved access and passed; this was an environment restriction,
not a skipped gate. Intermediate lint/test failures were corrected and all
final gates were observed green. Unrelated untracked files were left intact.
