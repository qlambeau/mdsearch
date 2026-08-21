---
id: US-015
title: "Embed collections at the selected model's embedding dimension"
type: user-story
status: approved
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: PRD-001
epic: EPIC-009
feature: 015-embedding-dimensions
related:
  - US-010
  - US-011
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want `mdsearch embed` to index collections at the selected model's embedding
dimension,
so that any supported fastembed model — including the advertised 1024-dimension
`bge-large-en-v1.5` and `multilingual-e5-large` — works instead of failing
with a storage-level dimension mismatch.

## Context And Value

The semantic index schema, the `EMBEDDING_DIMENSION = 384` constant, and the
`rebuild` dimension guard in the SQLite adapter are all pinned to 384
dimensions, while the CLI exposes a generic `--model` switch and the adapter
advertises 1024-dimension models through its friendly-name map. Selecting such
a model is accepted and resolved, but the very first collection rebuild then
fails deep in storage with a confusing "embedding dimension mismatch" error
that does not surface as a model-selection problem (OBS-002).

The product therefore over-advertises: valid, supported-looking model names
lead to a hard runtime failure instead of either working or being rejected up
front, and only one model class is actually usable, constraining evaluation
and quality work (ADR-004).

This story makes the index dimension-aware: each collection's vector table is
created at the dimension of the model that built it, the model name and
dimension are recorded in `semantic_index_state`, and reads validate the
recorded dimension so a change is detected explicitly rather than silently
misreported.

## Business Rules

- The semantic index stores vectors at the dimension of the model used for
  that collection's rebuild: the per-collection vector table is created at
  that model's dimension.
- The model name and its embedding dimension are recorded in
  `semantic_index_state` when a collection is (re)built.
- Rebuilding a collection with a model of a different dimension recreates the
  vector table at the new dimension and updates the recorded state — an
  explicit, detected change, never a hidden failure.
- Reads (the `hybrid` semantic leg and `status`) validate the stored vector
  dimension against the recorded state; a mismatch is a clear error, never
  silent wrong results.
- The default model remains `all-MiniLM-L6-v2` (384 dimensions); existing
  384-dimension databases remain fully usable.
- Databases whose state predates dimension recording keep working (treated as
  384 dimensions) until the next rebuild records the dimension.
- Collections share one active embedding dimension: the vector table is
  created at the active model's dimension, and a model switch recreates the
  table and rebuilds every embedded collection under the new model (REQ-010
  FR-007 semantics).
- The documented model matrix (name to dimension) reflects every supported
  fastembed model.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `embed --model bge-large-en-v1.5` is selected | I run `mdsearch embed` | The collection is indexed at 1024 dimensions; `hybrid` and `status` work |
| EX-002 | A collection was embedded at 1024 dimensions | I run `mdsearch embed` with the default model | The vector table is recreated at 384 dimensions; state records the new model and dimension |
| EX-003 | A collection was embedded | I run `mdsearch status` | The recorded embedding model and dimension are shown |
| EX-004 | The recorded dimension disagrees with the stored vectors | I run `hybrid` | A clear dimension error is reported; no silent results |
| EX-005 | A legacy database built before dimension recording | I run `hybrid` or `status` | Everything works unchanged; the next rebuild records the dimension |

## Acceptance Criteria

- Every supported fastembed model (including `bge-large-en-v1.5` and
  `multilingual-e5-large`) embeds, rebuilds, and searches without a dimension
  error.
- The vector table for a collection is created at the dimension of the model
  used for its rebuild, and the model name and dimension are recorded in
  `semantic_index_state`.
- Rebuilding a collection with a different-dimension model recreates the
  vector table at the new dimension and updates the recorded state.
- Reads (`hybrid` semantic leg, `status`) report a clear error when the stored
  vectors disagree with the recorded dimension, and never return silent wrong
  results.
- Legacy databases built before dimension recording remain fully usable.
- Regression scenarios are added to the `010-semantic-index` and
  `011-hybrid-search` feature packets.

## Scope Boundaries

### In Scope

- Dimension-aware vector table creation per collection in the SQLite adapter.
- Recording the model name and dimension in `semantic_index_state` at rebuild.
- Read-time dimension validation on the `hybrid` semantic leg and `status`.
- Documented model-to-dimension matrix for every supported fastembed model.
- Regression scenarios in the `010-semantic-index` and `011-hybrid-search`
  packets.

### Out Of Scope

- Changing the default embedding model.
- Coexisting collections at different dimensions: all collections share the
  active model's dimension.
- Adding non-fastembed embedding providers.
- Vector quantization, compression, or dimensionality reduction.
- Changing lexical indexing or hybrid fusion behavior.
- Other TODO.md observations (OBS-004, OBS-005, ...).

## Dependencies

- `US-010` (EPIC-004) provides the semantic index build/rebuild path whose
  dimension guard this story replaces.
- `US-011` (EPIC-004) provides the hybrid semantic leg that reads the stored
  vectors and gains read-time dimension validation.
- The schema artifacts `TABLE-008` (`semantic_index_state`) and `TABLE-009`
  (`embeddings`) and `DB-001` are revised for the recorded dimension.
- `ADR-004` evaluation remains valid; the change widens the usable model set.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | How are legacy state rows without a recorded dimension treated: inferred as 384, or marked unknown until the next rebuild? | No | TBD | Resolved: inferred as 384 |
| OQ-002 | How are per-collection dimensions physically stored when sqlite-vector tables fix one dimension per table? | No | TBD | Resolved: one shared vector table recreated at the active model's dimension; all collections share it |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable