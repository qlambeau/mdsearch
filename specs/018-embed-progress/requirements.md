---
id: REQ-018
title: "embed shows live ingestion progress on stderr requirements"
type: feature-requirements
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-018
related:
  - US-010
  - REQ-010
  - REQ-017
  - ADR-013
---

# Requirements

## Purpose And Actors

### Purpose

Make `mdsearch embed` visibly work during long ingestion runs: the embedding
phase reports per-file progress on stderr (collection name and files-completed
against the total) while the stdout report contract stays byte-identical, and
the database write phase shows a single status message. The feature completes
EPIC-012 (OBS-016).

### Actors And External Systems

- Developer-curator invoking `mdsearch embed` and watching the terminal.
- Coding-agent harness invoking `mdsearch embed` and reading stdout.
- The local embedding generator (fastembed adapter) and the semantic index
  store, unchanged in their contracts.

## Preconditions

- The user invokes `mdsearch embed` with the existing switches (`--collection`,
  `--model`, `--reranker`, `--download`, `--database`).
- The existing embed contracts from `REQ-010` — collection scoping, model
  resolution, download gating, per-collection outcomes, partial-failure
  handling, and stdout rendering — remain in force except for the added
  stderr progress.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Embed a collection | `embed [--collection NAME] ...` | Per-collection stdout summary as in `REQ-010`; per-file progress on stderr during the embedding phase | The collection has files and a built lexical index |
| Embed all collections | `embed` | Same, per collection; progress restarts per collection on stderr | Same per target collection |
| Already-current collection | `embed` | stdout "already current" line; no stderr progress | Fingerprint and model match the recorded state |
| Skipped collection | `embed` | stdout skipped line; no stderr progress | No files or no lexical index |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | During the embedding phase, `embed` shall report progress on stderr that advances per file within the collection being embedded, naming the collection and showing the completed file count against the total. | Must | US-018; Per-file progress is shown on stderr for a single collection |
| FR-002 | In a multi-collection run, progress shall be reported per collection: each collection's progress names that collection and restarts its own file counter. | Must | US-018; Multi-collection runs report progress per collection |
| FR-003 | Skipped collections (no files, no lexical index) and already-current collections shall produce no stderr progress output. | Must | US-018; An already-current collection produces no progress; A skipped collection produces no progress |
| FR-004 | The stdout report for `embed` shall be byte-identical to the pre-change output for the same input state: progress must never appear in or alter stdout. | Must | US-018; The stdout report is unchanged with progress enabled |
| FR-005 | The database write phase (storing the generated vectors) shall show a single status message and shall not run a progress bar. | Must | US-018; The database write phase shows a single status message |
| FR-006 | The `--download` phase shall behave exactly as before; progress reporting begins only once embedding starts. | Must | US-018; The --download path is unchanged |
| FR-007 | When a collection fails partway through embedding, its progress shall be finalized before the per-collection failure outcome is printed, and other collections in the run shall continue to report progress. | Must | US-018; A failing collection finalizes progress before the failure line |
| FR-008 | A failure while rendering progress (e.g. a closed stderr) shall not fail the run: progress rendering is best-effort and never changes the command outcome. | Must | US-018 (acceptance criteria; scope) |

## Postconditions And Invariants

- stderr progress and stdout outcomes describe the same run: the final progress
  state for a collection matches its outcome (embedded, failed, skipped, or
  already current).
- stdout carries exactly the `REQ-010` report; no progress text, escape
  sequences, or status messages appear on stdout.
- The embedding phase is the only phase with a progress bar; the write phase
  is a single message.
- The command operates offline unless `--download` is passed (unchanged).

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Collection with no embeddable files (zero passages) | No per-file progress; progress finalized immediately | Embedding outcomes as in `REQ-010` |
| Collection fails mid-embedding | Progress bar finalized, then the failure outcome line | Partial progress visible; failure reported as before |
| stderr closed or unwritable | Progress writes fail silently (best-effort) | Command outcome unchanged; no crash |
| Already-current or skipped collection | No progress output | stdout lines as before |
| `--download` in progress | No progress output until embedding starts | Download behavior unchanged |

## Quality Requirements

- No change to stdout content or ordering for any input state (byte-identical
  contract).
- Progress rendering adds no measurable latency to the embedding path (the
  embedding cost dominates) and no work to already-current/skipped paths.
- Progress updates are bounded: at most one update per completed file, so
  stderr volume is proportional to file count, not passage count.
- The progress text is deterministic and greppable (collection name and
  numeric counts), suitable for a human terminal without machine parsing
  obligations.

## Dependencies And Deferred Decisions

- `REQ-010` contracts stay in force; only the stderr side is added (R-SDD-05).
- The `indicatif` crate is added to the `app` crate (owner-approved new
  dependency; recorded in ADR-013).
- The progress hook placement — how the use case reports per-file progress and
  how the CLI renders it — is decided in design and ADR-013.
- The total file count per collection is derived from the passage set; files
  without passages are not part of the progress total (story OQ-001).

## Traceability

- Source story: `US-018` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-012, DEC-017)