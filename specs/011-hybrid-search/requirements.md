---
id: REQ-011
title: "Hybrid search with lexical-semantic fusion and cross-encoder re-ranking requirements"
type: feature-requirements
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: US-011
related:
  - US-010
  - US-007
  - US-006
  - DES-007
  - DES-010
  - ADR-004
  - ADR-006
  - DB-001
  - TABLE-004
  - TABLE-005
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Requirements

## Purpose And Actors

### Purpose

Provide a dedicated `mdsearch hybrid QUERY` command that retrieves candidate
passages from both the lexical (BM25) index and the semantic (vector) index,
fuses them into one ranked list with Reciprocal Rank Fusion, and re-ranks the
fused list with a local cross-encoder model so that conceptual queries that
keywords miss are answered with the best ordering. The command completes
EPIC-004.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- The local re-ranker model asset cache (read-only; provisioning happens
  through `mdsearch embed --reranker NAME --download`).

## Preconditions

- The user invokes `mdsearch hybrid QUERY`, with optional `--collection NAME`,
  `--limit N`, `--json`, `--no-rerank`, and `--database PATH`.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- The database exists.
- At least one in-scope collection has a built lexical index (built via
  `mdsearch collection update`), and any collection contributing to the
  semantic leg has a current semantic index (built via `mdsearch embed`).

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Hybrid search all collections | `QUERY`, optional `--limit N`, optional `--json`, optional `--no-rerank`, optional `--database PATH` | Ranked passage blocks (rank, file path, kind, text, ordering score) plus a summary line reporting the shown count; empty output when nothing matches | Query is free text and non-empty; `--limit` within 1..=100; every in-scope semantic index is current |
| Hybrid search one collection | `QUERY`, `--collection NAME`, optional `--limit N`, optional `--json`, optional `--no-rerank`, optional `--database PATH` | Same ranked output restricted to the named collection | `NAME` matched case-insensitively; the collection exists; its lexical index is built; its semantic index, if present, is current |
| Re-ranker fallback | `QUERY` with re-ranking on and the re-ranker model uncached | RRF-only results plus a warning that re-ranking was skipped | Re-ranking remains on; the run does not fail |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `mdsearch hybrid QUERY` shall return a single ranked list fused from the lexical (BM25) and semantic (cosine) candidate sets with Reciprocal Rank Fusion, keyed on the stable logical passage identity `(file_id, kind, position)`, and cut to `--limit`. | Must | US-011; Hybrid search returns a single fused ranked list |
| FR-002 | The hybrid query shall be free text: no FTS5 operators (`AND`, `OR`, `NOT`, quotes, or `prefix*`); the same string shall drive both the lexical leg and the semantic leg. | Must | US-011; Free-text query handling |
| FR-003 | An empty or whitespace-only query shall fail with a clear error. | Must | US-011; Fail on an empty query |
| FR-004 | The command shall re-rank the whole fused candidate pool with a local cross-encoder model by default, order the results by the re-ranker score, and then cut to `--limit`. | Must | US-011; Hybrid search returns a single fused ranked list; A both-leg match outranks single-leg matches |
| FR-005 | `--no-rerank` shall disable the re-ranking stage; the final order shall follow the fused RRF scores. | Must | US-011; --no-rerank orders results by the fused RRF score |
| FR-006 | If the re-ranker model assets are not cached locally and re-ranking is on, the command shall fall back to RRF-only ordering for that run and print a warning that re-ranking was skipped; with `--no-rerank`, no warning shall be printed. | Must | US-011; An uncached re-ranker falls back to RRF-only with a warning; --no-rerank with an uncached re-ranker produces no warning |
| FR-007 | A collection with both a built lexical index and a built semantic index shall contribute to both legs; a collection with a built lexical index but no semantic index shall contribute its lexical results only, and its passages shall still be re-ranked when the re-ranking stage runs. | Must | US-011; A collection without a semantic index contributes lexical results |
| FR-008 | When searching all collections, collections whose lexical index is not built and collections with no stored files shall be skipped silently and contribute nothing. | Must | US-011; Skip unbuilt and empty collections when searching all |
| FR-009 | When `--collection` targets a collection that does not exist or whose lexical index is not built, the command shall fail and report the reason. | Must | US-011; Report a missing collection for --collection; Report an unbuilt lexical index for --collection |
| FR-010 | If any in-scope collection's semantic index is stale (its stored file set changed since the last successful embed), the command shall fail and direct the user to run `mdsearch embed`. | Must | US-011; Fail when an in-scope semantic index is stale |
| FR-011 | Each result block shall report the rank, the file path, the passage kind (`body`, `title`, `tags`, `aliases`, or `summary`), the passage text, and the ordering score: the re-ranker score when the re-ranking stage ran, otherwise the fused RRF score. | Must | US-011; Hybrid search returns a single fused ranked list; --no-rerank orders results by the fused RRF score |
| FR-012 | When at least one result exists, the output shall end with a summary line reporting the number of results shown, never more than `--limit`. | Must | US-011; Cap results with --limit and report the shown count |
| FR-013 | With `--json`, each result shall include the re-ranker score (when the re-ranking stage ran), the fused RRF score, the BM25 score, the cosine similarity, the collection, file path, passage kind, passage text, and position. | Must | US-011; Emit JSON with per-leg, fused, and re-ranker scores |
| FR-014 | Results with equal ordering scores shall be ordered deterministically by collection name, then file path, then passage position. | Must | US-011; Hybrid search returns a single fused ranked list |
| FR-015 | When no passage matches the query, the output shall be empty. | Must | US-011; Produce empty output when nothing matches |
| FR-016 | `mdsearch hybrid` against a missing database shall fail semantically without creating a database file. | Must | US-011; Report a missing database without creating a file |
| FR-017 | The re-ranker model shall be provisioned through `mdsearch embed --reranker NAME --download`, which fetches its assets into the local cache and records a global re-ranker model setting; the re-ranker shall not store vectors. | Must | US-011; Re-ranker provisioning via embed |
| FR-018 | `--database PATH` shall select the database used by `mdsearch hybrid`. | Must | US-011; Report a missing database without creating a file |
| FR-019 | Before the semantic leg runs, the command shall validate each in-scope collection's recorded dimension (`semantic_index_state.dimension`, legacy NULL read as 384) against the active `embedding_dimension` setting; a disagreement shall fail the command with a clear dimension-mismatch error and no partial results. | Must | US-015; A dimension mismatch on the semantic leg reports a clear error |

## Postconditions And Invariants

- The command is read-only: it does not modify the indexes, the stored files,
  the collections, the re-ranker setting, or the database.
- The returned list contains at most `--limit` results, ordered by the ordering
  score (re-ranker when active, else fused RRF), with deterministic tie-breaking.
- Every returned passage is a real lexical passage; semantic-only candidates
  that appear in the fused pool but are not returned do not appear in the
  output.
- The command operates fully offline; it never initiates a network request.
- A hybrid search never runs against a stale semantic index: stale state is a
  pre-command failure, not a degraded run.
- Re-ranking never changes which passages are candidates, only their order.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Empty or whitespace-only query | Fail | Clear error |
| `--limit` out of 1..=100 | Fail | Clear error |
| Re-ranker assets not cached, re-ranking on | Fall back to RRF-only ordering for this run | Results plus a warning that re-ranking was skipped |
| Re-ranker assets not cached, `--no-rerank` | RRF-only ordering, no warning | Results with fused scores |
| Collection with lexical index but no semantic index | Contribute lexical results; passages re-ranked when stage 2 runs | Lexical results included |
| In-scope semantic index stale | Fail before returning results | Error directing the user to run `mdsearch embed` |
| `--collection` targets an unknown collection | Fail | Error that the collection was not found |
| `--collection` targets a collection with an unbuilt lexical index | Fail | Error that the index is not built |
| A recorded dimension disagrees with the active dimension | Fail before returning results | Clear dimension-mismatch error; no partial results |
| Unbuilt lexical index or empty collection in all-mode | Skip silently | No contribution |
| No passage matches | Empty output | No output |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |

## Quality Requirements

- The command shall operate offline by default and shall not require a network
  service at runtime.
- Re-ranking is driven by an explicit local model; re-ranker assets come from
  the local cache or are provisioned ahead of time through `mdsearch embed
  --reranker NAME --download`.
- The fused and re-ranked ordering shall be tunable (RRF `k`, oversample
  factor, re-ranker model) and validated through the ADR-004 evaluation
  framework; the tuning values themselves are deferred to design and not fixed
  by this contract.
- Query latency target remains the soft `TBD` from PRD-001; the two-stage
  pipeline must stay fast enough for harness context-filling.

## Dependencies And Deferred Decisions

- The lexical leg reuses the FTS5/BM25 search capability from `US-007` and its
  store contracts.
- The semantic leg reads the per-passage vectors stored by `US-010`
  (TABLE-009) keyed to the lexical passages.
- Stale detection reads the per-collection `semantic_index_state`
  (TABLE-008) and compares the stored file-set fingerprint.
- The cross-encoder re-ranker model and whether it is hosted by `fastembed` or
  a new dependency is deferred to design and recorded in an ADR (OQ-001).
- The concrete RRF `k` and per-leg oversample factors are deferred to design
  and tuned through ADR-004 (OQ-002).

## Traceability

- Source story: `US-011` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
