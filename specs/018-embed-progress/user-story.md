---
id: US-018
title: "embed shows live ingestion progress on stderr"
type: user-story
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: PRD-001
epic: EPIC-012
feature: 018-embed-progress
related:
  - US-010
  - US-017
---

# User Story

## Story Card

As a developer-curator,
I want `mdsearch embed` to show live progress — files embedded against the
total, per collection — on stderr during long ingestion runs,
so that I can tell the tool is working and estimate how far a rebuild is
instead of staring at a silent terminal.

## Context And Value

`mdsearch embed` performs the embedding pass as an all-or-nothing batch: it
loads every passage of every targeted collection, embeds them all in one
blocking adapter call, then writes every vector row in one transaction
(OBS-016). During that time the terminal is silent — no progress bar, no
per-file or per-passage counter, not even a per-collection line. Only when the
whole run completes does the CLI print one line per collection.

At PRD scale (100–5,000 documents, 100–300 page files, CPU inference via
fastembed/ONNX) a single collection rebuild can take many minutes with no way
for the user to tell whether the tool is working, stuck, or nearly done — and a
mid-run failure wastes the entire preceding work with no partial feedback.

This story adds per-file progress feedback on stderr during the embedding
phase, keeps the stdout report contract unchanged, and shows a single status
message for the database write phase. The model-download phase keeps its own
existing behavior and is out of scope.

## Business Rules

- Progress is reported per file within each collection being embedded: the
  current file and the total file count for that collection.
- Progress is always written to stderr; stdout carries exactly the same report
  as before (per-collection outcome lines, unchanged text and shape).
- Multi-collection runs (`EmbedScope::All`) report progress per collection,
  restarting the per-file counter for each collection, and name the collection.
- Skipped collections (no files, no lexical index) and already-current
  collections produce no progress output.
- The embedding phase is the progress-bearing phase; the database write phase
  shows a single status message and does not run a progress bar.
- The model-download phase (`--download`) is not part of this story; its
  behavior is unchanged.
- Rendering uses the `indicatif` progress-bar crate (new dependency in the
  `app` crate, approved by the owner in this session; recorded in an ADR).

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | One collection "Notes" with 340 files needs embedding | I run `mdsearch embed` | stderr shows per-file progress ("file 12/340") that advances during the run |
| EX-002 | Several collections need embedding | I run `mdsearch embed` (all collections) | stderr shows progress per collection, naming each collection and restarting its file counter |
| EX-003 | A collection is already current | I run `mdsearch embed` | No progress output for that collection; the stdout report says "already current" |
| EX-004 | Embedding finishes | I run `mdsearch embed` | The stdout report is exactly as before (e.g. "collection \"Notes\": embedded 500 passage(s)") |
| EX-005 | A collection fails mid-run | I run `mdsearch embed` | Progress stops for that collection; the stdout report carries the per-collection failure line as before |

## Acceptance Criteria

- `mdsearch embed` displays per-file progress on stderr during the embedding
  phase, including the collection name and the files-completed/total counts.
- Progress appears for every collection being embedded, including in
  multi-collection runs.
- The stdout report is byte-identical to the pre-change output for the same
  input state.
- No progress output appears for skipped or already-current collections.
- The database write phase shows a single status message without a progress
  bar.
- The `--download` path and its output are unchanged.

## Scope Boundaries

### In Scope

- Per-file progress rendering on stderr for the embedding phase, per
  collection, via the `indicatif` crate.
- A single status message for the database write phase.
- Application-layer progress hooking (the use case reports progress; the CLI
  renders it).

### Out Of Scope

- Progress for the model-download phase.
- Machine-readable progress output (embed has no JSON mode).
- Per-passage or per-batch granularity.
- Progress on `search`, `hybrid`, `update`, or `index status`.
- Line-per-file output or other rendering styles.
- Other TODO.md observations (OBS-004, OBS-005, ...).

## Dependencies

- `US-010` (EPIC-004) provides the embed flow whose batch embedding this story
  splits into per-file progress.
- `US-017` (EPIC-011) is implemented: the cache and availability work this
  story builds on top of.
- The `indicatif` crate is added to the `app` crate (owner-approved new
  dependency; recorded in an ADR).

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Are per-file counts taken from the stored passages (files with ≥1 passage in the collection), including files that yield zero passages? | No | TBD | Resolved: files are counted from the passage set; files with no passages are not part of the progress total |
| OQ-002 | Does a failed per-collection run leave a partial progress state on stderr? | No | TBD | Resolved: the progress bar is finalized before the outcome line is printed; no half-drawn state |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable