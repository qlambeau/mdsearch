---
id: US-011
title: "Hybrid search with lexical-semantic fusion and cross-encoder re-ranking"
type: user-story
status: approved
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: PRD-001
epic: EPIC-004
feature: 011-hybrid-search
related:
  - US-010
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want to search with `mdsearch hybrid QUERY`, a two-stage pipeline that fuses
lexical and semantic results and then re-ranks them with a local cross-encoder,
so that conceptual queries that keywords miss are still answered with the best
ordering.

## Context And Value

`US-010` built the per-passage semantic index with `mdsearch embed`, and `US-007`
added the lexical `mdsearch search` command ranked by BM25. Each index alone is
incomplete: lexical search misses conceptual paraphrases, and pure semantic
search can miss exact-symbol and verbatim-phrase queries. This story completes
EPIC-004 by adding a dedicated hybrid search command that retrieves candidates
from both indexes, fuses them into one ranked list, and re-ranks the fused list
with a local cross-encoder model for the final order.

Both personas benefit: the developer-curator gets a single command that answers
both keyword and conceptual queries, and the harness can inject the fused
top-K passages with their provenance into an LLM prompt (JSON output exposes
the per-leg scores for later tuning via the ADR-004 evaluation framework).

## Business Rules

- The command is `mdsearch hybrid QUERY`, with optional `--collection NAME`,
  `--limit N`, `--json`, `--no-rerank`, and `--database PATH`.
- `--limit` defaults to 10 and accepts values from 1 through 100 inclusive.
- The query is free text: there are no FTS5 operators (`AND`, `OR`, `NOT`,
  quotes, or `prefix*`). The same free-text string drives both the lexical leg
  and the semantic leg.
- An empty or whitespace-only query fails with a clear error.
- The pipeline has two stages:
  - **Stage 1 (retrieval and fusion):** the lexical (BM25) leg and the semantic
    (cosine vector) leg each retrieve an oversampled candidate pool; the two
    ranked lists are fused with Reciprocal Rank Fusion (RRF), keyed on the
    stable logical passage identity `(file_id, kind, position)`.
  - **Stage 2 (re-ranking):** the whole fused candidate pool is re-scored by a
    local cross-encoder re-ranker model; the final order follows the re-ranker
    score, and the list is cut to `--limit`.
- Re-ranking is always on by default; `--no-rerank` disables stage 2, so the
  final order follows the fused RRF scores.
- If the re-ranker model assets are not cached locally and re-ranking is on,
  the command falls back to RRF-only fusion for that run and prints a warning
  that re-ranking was skipped; it does not fail.
- A collection with both a built lexical index and a built semantic index
  contributes to both legs.
- A collection with a built lexical index but no semantic index contributes
  its lexical results only (lexical-only fallback); its passages are still
  re-ranked when stage 2 runs.
- When searching all collections, collections whose lexical index is not built
  are skipped silently; a collection with no stored files contributes nothing.
- When `--collection` targets a collection that does not exist or whose lexical
  index is not built, the command fails and reports the reason.
- If any in-scope collection's semantic index is stale (its stored file set
  changed since the last successful `embed`), the command fails and directs the
  user to run `mdsearch embed` first.
- Each result block reports the rank, the file path, the passage kind (`body`,
  `title`, `tags`, `aliases`, or `summary`), the passage text, and the ordering
  score: the re-ranker score when stage 2 ran, otherwise the fused RRF score.
- When at least one result exists, the output ends with a summary line
  reporting the number of results shown (never more than `--limit`).
- With `--json`, each result includes the re-ranker score (when stage 2 ran),
  the fused RRF score, the BM25 score, the cosine similarity, the collection,
  file path, passage kind, passage text, and position.
- Equal ordering scores are ordered deterministically by collection name, then
  file path, then passage position.
- When no passage matches the query, the output is empty.
- If the selected database does not exist, the command fails and reports the
  database does not exist without creating a file.
- Re-ranker model provisioning happens through `mdsearch embed`: `--reranker
  NAME` selects the re-ranker model and `--download` fetches its assets; the
  re-ranker is a local cache entry plus a recorded global setting, not stored
  vectors.
- Exact human-readable wording of errors, warnings, and score formatting is not
  part of this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `Notes` has a built lexical and semantic index with a passage about borrowing | I run `mdsearch hybrid "how do I borrow this"` | The borrowing passage is returned and ordered by the re-ranker score |
| EX-002 | `Notes` has a built lexical index but no semantic index | I run `mdsearch hybrid "borrowing"` | Lexical matches from `Notes` are returned with a re-ranker order and a warning that re-ranking was skipped if the re-ranker is uncached |
| EX-003 | A passage matches both legs | I run `mdsearch hybrid QUERY` | The both-leg passage outranks passages matched by a single leg |
| EX-004 | The re-ranker assets are not cached | I run `mdsearch hybrid QUERY` | RRF-only results are returned with a warning that re-ranking was skipped |
| EX-005 | The re-ranker assets are not cached | I run `mdsearch hybrid QUERY --no-rerank` | RRF-only results are returned with no warning |
| EX-006 | `Notes`'s semantic index is stale (files changed since embed) | I run `mdsearch hybrid QUERY` | The command fails and directs me to run `mdsearch embed` |
| EX-007 | No collection named `Journal` exists | I run `mdsearch hybrid QUERY --collection Journal` | The command fails and reports the collection was not found |
| EX-008 | `Notes` stores files but its lexical index was never built | I run `mdsearch hybrid QUERY --collection Notes` | The command fails and reports the index is not built |
| EX-009 | A query is empty | I run `mdsearch hybrid ""` | The command fails with a clear error |
| EX-010 | No passage matches the query | I run `mdsearch hybrid zzzznotaword` | The output is empty |
| EX-011 | `--limit 5` is supplied | I run `mdsearch hybrid QUERY --limit 5` | At most 5 result blocks are shown and the summary reports the shown count |
| EX-012 | No database exists at the selected path | I run `mdsearch hybrid QUERY --database PATH` | The command fails and reports the database does not exist |
| EX-013 | `--json` is supplied | I run `mdsearch hybrid QUERY --json` | Each result includes re-ranker, fused, BM25, and cosine scores plus provenance |

## Acceptance Criteria

- `mdsearch hybrid QUERY` returns a single ranked list fused from lexical and
  semantic candidates with Reciprocal Rank Fusion, keyed on the logical passage
  identity, and cut to `--limit`.
- Re-ranking by a local cross-encoder is on by default and disabled by
  `--no-rerank`; an uncached re-ranker falls back to RRF-only with a warning
  rather than failing.
- Free-text queries are accepted; empty and whitespace-only queries fail with a
  clear error.
- Collections with both indexes contribute both legs; collections with only a
  lexical index contribute lexically and their passages are re-ranked when
  stage 2 runs.
- All-mode skips unbuilt lexical indexes and empty collections silently;
  `--collection` targeting an unknown collection or an unbuilt lexical index
  fails.
- A stale in-scope semantic index fails the command with guidance to run
  `mdsearch embed`.
- Result blocks show the ordering score; `--json` exposes re-ranker, fused,
  BM25, and cosine scores with provenance.
- The summary line reports the number of results shown (≤ `--limit`).
- Equal scores tie-break by collection name, then file path, then passage
  position.
- No matches produce empty output; a missing database fails without creating a
  file.
- Re-ranker assets are provisioned via `mdsearch embed --reranker NAME
  --download` and recorded as a global setting, not stored vectors.

## Scope Boundaries

### In Scope

- The dedicated `mdsearch hybrid` command with `--collection`, `--limit`,
  `--json`, `--no-rerank`, and `--database` switches.
- Two-stage retrieval: oversampled lexical and semantic candidate pools, RRF
  fusion, cross-encoder re-ranking of the whole fused pool, and cutting to
  `--limit`.
- Free-text query handling shared by both legs.
- Lexical-only fallback for collections without a semantic index.
- Stale-semantic-index detection and failure with `embed` guidance.
- Uncacheable re-ranker fallback with warning.
- Human-readable and `--json` output with per-leg and fused scores.
- Re-ranker provisioning through `mdsearch embed` (`--reranker` + `--download`)
  and a recorded global re-ranker model setting.

### Out Of Scope

- Changing the lexical `mdsearch search` command or the `embed` vector index
  behavior.
- FTS5 operator syntax in hybrid queries.
- Tuning the RRF `k`, oversample factor, or fusion weights (the ADR-004
  evaluation framework tunes these later).
- External re-ranker services; the re-ranker is local-only.
- Related-concept links and entity graphs (EPIC-005).
- JSON shape changes to the existing `search` command (EPIC-006).

## Dependencies

- `US-010` provides the built per-passage semantic index and the stale-index
  detection state (`semantic_index_state`) that hybrid search reads.
- `US-007` provides the lexical search port and result contracts that the
  lexical leg reuses or mirrors.
- `US-006` provides the per-passage FTS5 index and `passage_files` rows that
  the logical passage identity joins on.
- `fastembed` (or a design-time alternative) provides the local cross-encoder
  re-ranker (ADR-006 records the embedding side; a new ADR covers the re-ranker
  model and any new dependency).
- ADR-004's evaluation framework tunes the fusion parameters and validates
  re-ranked quality.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Which local cross-encoder model is the re-ranker default, and can `fastembed` host it without a new dependency? | Yes, for design | TBD | Open |
| OQ-002 | What are the concrete RRF `k` and per-leg oversample factor defaults? | No | TBD | Open (tuned later via ADR-004) |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
