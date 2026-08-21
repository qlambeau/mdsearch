---
id: REQ-015
title: "Embed collections at the selected model's embedding dimension requirements"
type: feature-requirements
status: draft
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: US-015
related:
  - US-010
  - US-011
  - REQ-010
  - REQ-011
  - REQ-006
  - ADR-004
  - DB-001
  - TABLE-008
  - TABLE-009
---

# Requirements

## Purpose And Actors

### Purpose

Make the semantic index dimension-aware: every supported fastembed model — including the advertised 1024-dimension `bge-large-en-v1.5` and `multilingual-e5-large` — embeds, rebuilds, and searches without a dimension error. Each collection's vectors are stored at the dimension of the model that built them, the model name and dimension are recorded in `semantic_index_state` at rebuild, and reads (`hybrid` semantic leg, `index status`) validate the recorded dimension so a mismatch is a clear error rather than silent wrong results. The feature completes EPIC-009.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit `--database PATH` override.
- The local embedding-model asset cache.
- The network, used only when `--download` is passed.

## Preconditions

- The user invokes `mdsearch embed`, `mdsearch hybrid`, or `mdsearch index status` with the respective command's existing switches.
- The database exists; the lexical index exists for any collection being embedded or searched.
- The embed command's existing model selection, download gating, and atomic-rebuild contracts from `REQ-010` remain in force.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Embed with a model | `--model NAME` (or recorded/default model) | Per-collection summary as in `REQ-010`, including the model used | The model is supported and cached (or `--download` passed); the collection's vectors are stored at that model's dimension |
| Rebuild under a different model | `--model NAME` differing from the recorded model | Rebuilt per-collection vectors at the new dimension; state records the new model and dimension | Model change detection as in `REQ-010` FR-007 |
| Read dimension validation | `hybrid QUERY` (semantic leg) | Ranked results as in `REQ-011` | Stored vectors match the dimension recorded for the collection |
| Status read | `mdsearch index status` | Per-collection lines including the recorded semantic model and dimension when embedded | A semantic state row exists for the collection |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | Every supported fastembed model shall embed a collection successfully at its own output dimension, including the 1024-dimension `bge-large-en-v1.5` and `multilingual-e5-large`; no supported model shall fail with a dimension error. | Must | US-015; Every supported model embeds at its own dimension |
| FR-002 | A collection's stored vectors shall be at the dimension of the model used for its last successful rebuild. | Must | US-015; A 1024-dimension model embeds and searches successfully |
| FR-003 | Rebuilding under a model of a different dimension shall recreate the shared vector table at the new dimension and rebuild every embedded collection into it at the new dimension, updating each collection's recorded state — an explicit, detected change, never a hidden failure. | Must | US-015; Rebuilding with a different-dimension model recreates the index |
| FR-004 | A successful rebuild shall record the embedding model name and its dimension in `semantic_index_state` for the collection. | Must | US-015; Status reports the recorded embedding model and dimension |
| FR-005 | `mdsearch index status` shall report, for each collection with a recorded semantic state, the embedding model and the dimension its vectors were built at. | Must | US-015; Status reports the recorded embedding model and dimension |
| FR-006 | The `hybrid` semantic leg shall validate the stored vector dimension against the recorded dimension; on a mismatch the command shall fail with a clear dimension error and return no partial results. | Must | US-015; A dimension mismatch on the semantic leg reports a clear error |
| FR-007 | A semantic state row without a recorded dimension (legacy database) shall be treated as 384 dimensions: `hybrid` and `index status` keep working unchanged. | Must | US-015; A legacy semantic index without a recorded dimension keeps working |
| FR-008 | The next successful rebuild of a legacy-state collection shall record its model and dimension. | Must | US-015; A legacy semantic index without a recorded dimension keeps working |
| FR-009 | The default embedding model shall remain `all-MiniLM-L6-v2`; the global-model switch semantics of `REQ-010` (FR-006, FR-007) are unchanged, except that a model switch rebuilds each collection at the new model's dimension. | Must | US-015; Rebuilding with a different-dimension model recreates the index |
| FR-010 | All other `embed`, `hybrid`, and `index status` contracts — collection scoping, download gating, atomic rebuild, per-collection failure handling, summary output, staleness detection, and missing-database behavior — shall remain unchanged. | Must | US-015 (scope boundaries) |

## Postconditions And Invariants

- After a successful rebuild, every stored vector for the collection has exactly the dimension recorded in its `semantic_index_state` row.
- The recorded dimension equals the model's documented output dimension.
- All collections with recorded semantic state share one active dimension: the shared vector table's dimension, the recorded state dimensions, and the active model's output dimension agree.
- The `hybrid` semantic leg never returns partial or mismatched results: a dimension disagreement is a pre-return failure.
- Existing 384-dimension databases and legacy state rows remain fully usable without migration.
- The command operates offline unless `--download` is passed; reads never modify state.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Recorded dimension disagrees with stored vectors on `hybrid` | Fail before returning results | Clear dimension-mismatch error; no partial results |
| Legacy semantic state without a recorded dimension | Treated as 384 dimensions | `hybrid` and `index status` work unchanged |
| Rebuild under a different-dimension model | Vectors recreated at the new dimension; state updated | New dimension in effect after the run |
| Unsupported model name | Fail before any collection work (unchanged `REQ-010` FR-008) | Clear error naming the model |
| Rebuild storage failure | Rolled back atomically (unchanged `REQ-010` FR-005) | Previous vectors intact |
| The database does not exist | Fail without creating a file (unchanged) | Output communicates the database does not exist |

## Quality Requirements

- Dimension is recorded explicitly and read-time validation is deterministic: reads never depend on model-name parsing or engine heuristics.
- No additional latency beyond the existing `hybrid`/`status` paths: validation is a per-vector-length check against a recorded integer.
- The documented model matrix (name to output dimension) matches the adapter's supported set, so advertised models always work.
- ADR-004 evaluation remains valid; the change widens the usable model set rather than altering ranking.

## Dependencies And Deferred Decisions

- `semantic_index_state` (`TABLE-008`) gains a recorded dimension alongside the existing `model` column.
- The physical vector layout is decided: one shared `embeddings` table recreated at the active model's dimension on model switch, with all collections sharing that dimension; recorded in an ADR (story OQ-002).
- `REQ-010` and `REQ-011` are revised as part of this feature so the dimension-aware behavior and the existing contracts stay in lockstep (R-SDD-05); `REQ-006` (the `index status` command) is revised for the semantic status line.
- The default model and supported-model set remain as in `REQ-010`/design records.

## Traceability

- Source story: `US-015` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-009, DEC-014)