---
id: REQ-020
title: "CLI help, diagnostics, and global options requirements"
type: feature-requirements
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved REQ-020 as written in chat on 2026-09-30"
parent: US-020
depends_on: []
requires: []
blockers: []
related: []
---

# Requirements

## Purpose And Actors

### Purpose

Make CLI discovery and scripted invocations reliable for the first redesign
slice, preserving the current command grammar until subsequent slices replace it.

### Actors And External Systems

- Developer-curators and coding agent harnesses invoking the binary.
- Local filesystem, selected database, HOME, and configured model cache paths.

## Preconditions

- Help, version, and argument diagnostics require no database or HOME.
- Operational commands retain their existing collection and index prerequisites.
- The parent PRD-001 and US-020 story are approved.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Information | Help at any command depth or top-level version | stdout information; exit 0 | No HOME requirement |
| Invalid invocation | Unknown options, missing arguments, invalid numeric limits | stderr diagnostic; exit 2 | Existing argument rules and inclusive limit range |
| Operation | Existing command arguments, global optional database path | Existing success output or stderr failure; exit 1 on operational failure | Existing command prerequisites |
| Search scope and size | `-c`/`--collection`, `-n`/`--limit` | Scoped results, default limit 10 | Limit 1–100 |
| Ingestion | Existing paths with optional `--skip-unreadable` | Existing ingestion result and skipped count | `--force` is rejected |

## Functional Requirements

All requirements below are Must priority and trace to US-020.

| ID | Requirement | Traceability (Scenario) |
| --- | --- | --- |
| FR-001 | Help at every command depth and top-level version shall write information to stdout, leave stderr empty, and exit 0 without requiring HOME. | Informational commands work without HOME |
| FR-002 | Argument errors shall write diagnostics to stderr, leave stdout empty, and exit 2, including when HOME is absent. | Argument errors use stderr and exit code 2; Search limit boundaries remain enforced; The old force switch is rejected |
| FR-003 | Operational failures shall write diagnostics to stderr, leave stdout empty, and exit 1. | Operational failures use stderr and exit code 1; A missing HOME is diagnosed only when a default path needs it |
| FR-004 | `--database PATH` shall be valid before, between, and after subcommands, selecting the supplied path consistently. | Database selection is global |
| FR-005 | HOME shall be resolved only when a default database or model-cache path needs it. An explicit database path shall allow operations that do not need a default model-cache path to work without HOME. | Database selection is global; A missing HOME is diagnosed only when a default path needs it |
| FR-006 | Every applicable collection switch shall accept `-c` as well as `--collection`; search and hybrid limits shall accept `-n` as well as `--limit`. | Collection and limit short switches select search results |
| FR-007 | Search limits shall retain default 10 and inclusive range 1–100. | Search limit boundaries remain enforced; Search uses a default limit of ten |
| FR-008 | Every command's help shall describe its purpose and effects, applicable prerequisites and defaults, and contain an invocation example. | Command help describes use and examples |
| FR-009 | Ingestion commands shall replace the CLI spelling `--force` with `--skip-unreadable`, preserving the existing unreadable-path handling of that switch in this slice. | The ingestion switch is named for unreadable-file handling; The old force switch is rejected |
| FR-010 | README and command help shall accurately describe database option placement, short switches, exit codes, and unreadable-file skipping; skipping shall not be described as forced reprocessing. | Ingestion documentation matches the switch behavior; Command help describes use and examples |

## Postconditions And Invariants

- Informational commands and invalid argument parsing do not open a database,
  ingest files, or initiate model downloads.
- Database contents, indexing, query semantics, and successful retrieval contracts
  are preserved by this invocation and diagnostics slice.
- Current positional collection arguments remain positional; FR-006 applies to
  existing collection switches.
- Unreadable directory handling, retained stored content, registered sources,
  and combined atomic indexes are delivered in the subsequent indexing slices.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| HOME absent, information requested | No default path is resolved | stdout information; exit 0 |
| HOME absent, invalid invocation | Argument diagnostic takes precedence | stderr diagnostic; exit 2 |
| HOME absent, default database required | Fail before database access | stderr home-directory diagnostic; exit 1 |
| Explicit missing database for read operation | Fail without creating it | stderr missing-database diagnostic; exit 1 |
| Limit 0 or 101 | Reject argument | stderr diagnostic; exit 2 |
| Limit 1 or 100 | Accept argument | Results bounded by the supplied limit |
| Unreadable path with renamed switch | Apply existing skip-and-continue behavior | Skipped count in ingestion report |
| Old `--force` spelling | Reject argument | stderr diagnostic; exit 2 |

## Quality Requirements

- Actual subprocess acceptance tests shall assert exit codes and output streams,
  including informational and argument-error invocations without HOME.
- Tests shall isolate databases and environment per subprocess and require no network.
- No dependency, workspace member, or architectural layer shall be introduced.
- Implementation shall satisfy the Constitution and record observed RED/GREEN
  evidence and `cargo xtask ci` / `cargo xtask eval` output via verify-feature.

## Dependencies And Deferred Decisions

- Existing dependencies and architecture are sufficient; technical implementation
  choices belong to the design after approval of this contract.
- The subsequent planned slices own persisted sources, semantic configuration,
  model management, unified retrieval, graph scope, status, and final grammar migration.
- No blocking product questions remain.

## Traceability

- Source story: [US-020](user-story.md).
- Executable scenarios: [scenarios.feature](scenarios.feature).
- Parent PRD: [PRD-001](../prds/PRD-001.md), EPIC-006.
- Engineering authority: [CONSTITUTION.md](../CONSTITUTION.md).
