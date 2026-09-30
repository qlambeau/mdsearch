---
id: US-020
title: "CLI help, diagnostics, and global options"
type: user-story
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved the proposed US-020 story in chat on 2026-09-30"
parent: PRD-001
epic: EPIC-006
feature: 020-cli-help-diagnostics
depends_on: []
requires: []
blockers: []
related: []
---

# User Story

## Story Card

As a developer-curator or coding agent harness,
I want descriptive help, predictable diagnostics, and consistent database options,
so that I can discover commands and reliably interpret their results.

## Context And Value

The first slice of the approved CLI redesign makes informational commands and
scripted invocations reliable. Help and version currently depend on HOME and
appear to fail; database selection is restricted to leaf commands; ingestion
documentation describes forced reprocessing although the switch skips unreadable
paths. This slice fixes those observable contracts before the indexing and
retrieval redesigns.

## Business Rules

- Help and version print to stdout and exit 0, including without HOME.
- Argument errors print to stderr and exit 2; operational failures exit 1.
- `--database PATH` works at every command depth. HOME is resolved only when a
  default path needs it.
- Applicable collection switches support `-c`; search limits support `-n`, with
  default 10 and inclusive range 1–100.
- Every command explains its purpose, prerequisites, defaults, and effects, with
  examples.
- `--skip-unreadable` replaces `--force`; documentation accurately describes
  skipping unreadable files.
- Subprocess tests verify exit codes and output streams.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | HOME is absent | Run `mdsearch --help` or `mdsearch --version` | Informational output on stdout, empty stderr, exit 0 |
| EX-002 | An invalid option | Invoke the binary | Diagnostic on stderr, exit 2 |
| EX-003 | An unavailable database | Run a command that requires that database | Operational diagnostic on stderr, exit 1 |
| EX-004 | An explicit database path | Place `--database PATH` before, between, or after subcommands | The command selects the same database |
| EX-005 | A searchable collection | Run `search rust -c Notes -n 1` | Select Notes and return at most one result |
| EX-006 | An unreadable file among ingestion paths | Invoke ingestion with `--skip-unreadable` | Skip the unreadable file using the existing ingestion behavior |

## Acceptance Criteria

- Help/version output, exit codes, and HOME independence satisfy the business rules.
- Invalid arguments and operational failures use the specified stream and exit code.
- Global database placement and short collection/limit switches work consistently.
- Limits accept 1 and 100 and reject 0 and 101; the default remains 10.
- Command help contains descriptions and examples; documentation reflects actual
  prerequisites, defaults, effects, and unreadable-file handling.
- `--skip-unreadable` replaces the misleading `--force` spelling.
- Actual subprocess tests verify these contracts.

## Scope Boundaries

### In Scope

- Help, diagnostics, global database selection, short option spellings, and
  ingestion switch/documentation corrections.
- Equivalent behavior in the current command grammar during this first slice.

### Out Of Scope

- Persistent source registration and discovery.
- Semantic configuration, combined atomic updates, and global model management.
- Unified retrieval, explicit retrieval IDs, exact retrieval bytes, graph scope,
  and richer status reporting.
- The final breaking command grammar and old-to-new mapping, delivered in later slices.
- New dependencies, workspace members, or architectural layers.

## Dependencies

- The approved PRD-001 EPIC-006 defines the developer-curator and harness actors.
- Existing collection, ingestion, indexing, and retrieval behavior provides the
  operational commands; this slice changes their invocation and diagnostics.

## Open Questions

None. The user approved this bounded story on 2026-09-30.

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
