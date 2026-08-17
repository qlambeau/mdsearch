---
id: TASK-008
title: "Show passage positions and machine-readable JSON for search implementation tasks"
type: implementation-tasks
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: US-008
related:
  - REQ-008
  - DES-008
  - US-007
  - US-006
---

# Tasks

## Implementation Approach

Implement the smallest complete path for positioned, machine-readable search
output: give `Passage` a byte offset in the domain, record that offset in the
schema, compute byte and line ranges in the search store, and render them in
both the human header and a new `--json` output. Keep `REQ-008` and the state
flow in `DES-008` authoritative.

All Rust work must obey `specs/CONSTITUTION.md`. Read it before the first edit,
write tests before implementation, and do not add dependencies, workspace
members, layers, unsafe code, or lint suppressions without the required approval
and justification.

This slice introduces a schema migration to version 4 (an added `byte_offset`
column) but no new dependency or workspace member. It does not implement file
retrieval by name/ID, related-concept links, or JSON for other commands.

## Ordered Tasks

- [x] **TASK-008-1:** Give `Passage` a `byte_offset` and have `segment_passages`
      compute it (body paragraphs structural offsets; frontmatter fields via
      their `key:` line).
  - Depends on: None
  - Verification: `cargo test -p kv-domain` passes; tests cover body paragraphs,
    frontmatter fields, CRLF content, and empty files.

- [x] **TASK-008-2:** Add the `Position` value (`byte_offset`, `byte_length`,
      `line_start`, `line_end`) to the search port and expose it on
      `SearchResult`.
  - Depends on: TASK-008-1
  - Verification: `cargo test -p kv-application` passes.

- [x] **TASK-008-3:** Migrate to schema version 4 (guarded `ALTER TABLE` adding
      `byte_offset` to `passage_files`), record offsets in `reconcile`, and have
      `SqliteLexicalSearchStore` compute `Position` from the stored offset, text
      length, and file content, with a pre-v4 fallback.
  - Depends on: TASK-008-2
  - Verification: Store integration tests confirm the v4 migration, byte and
    line ranges, the pre-v4 fallback, and position correctness in results.

- [x] **TASK-008-4:** Render `PATH:START-END` in the human search block header
      and serialize `--json` output as one object with query, scope, limit,
      total, and a `results` array (empty-JSON for zero matches); keep errors on
      stderr.
  - Depends on: TASK-008-3
  - Verification: CLI acceptance tests mapped from `scenarios.feature` confirm
    the human header, the JSON shape, empty-JSON output, and error behavior in
    both modes.

- [x] **TASK-008-5:** Execute the approved Gherkin scenarios and the Rust
      constitution gates.
  - Depends on: TASK-008-4
  - Verification: Every scenario in `scenarios.feature` passes, and
      `cargo xtask ci` passes with observed output (fmt, clippy `-D warnings`,
      test, doc, deny, and `llvm-cov` with the line-coverage threshold).

## Test And Verification Plan

- [ ] Unit checks: `byte_offset` across segmentation edge cases.
- [ ] Application checks: `Position` on search results.
- [ ] Integration checks: v4 migration, byte/line ranges, and pre-v4 fallback.
- [ ] CLI checks: human header, `--json` shape, empty-JSON, and errors.
- [ ] Gherkin scenarios: `scenarios.feature`.
- [ ] Non-functional checks: read-only search, offline execution, and no new
      workspace members or dependencies.
- [ ] Constitution checks: `cargo xtask ci` and the Definition of Done gates.
- [ ] Regression check: confirm no file-retrieval, link, or other-command JSON
      behavior is added.

## Rollout And Recovery

### Rollout

Ship the schema-v4 migration and the extended `search` output in the single
compiled binary. Migration applies idempotently on the next `collection update`;
existing databases gain `byte_offset` on the passage mapping table.

### Recovery

Search is read-only and writes nothing; re-running with corrected arguments
retries safely. A database not yet migrated to schema version 4 reports
positions with an unknown line range rather than failing. A missing database
fails without creating a file.

## Definition Of Done

- [x] All tasks are complete.
- [x] Automated unit, integration, and CLI checks pass.
- [x] The executable scenarios pass.
- [x] The Rust constitution's tooling gates and Definition of Done checklist pass.
- [x] Offline and no-server constraints are verified.
- [x] Relevant specifications are updated if implementation details require clarification.
- [x] No out-of-scope file-retrieval, link, or other-command JSON behavior was added.
- [x] Operational or documentation changes are complete.