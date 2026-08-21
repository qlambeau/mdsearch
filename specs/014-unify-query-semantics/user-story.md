---
id: US-014
title: "Unify literal free-text query semantics across lexical and hybrid search"
type: user-story
status: approved
created: 2026-08-21
updated: 2026-08-21
owner: TBD
parent: PRD-001
epic: EPIC-008
feature: 014-unify-query-semantics
related:
  - US-007
  - US-011
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want `search` and `hybrid` to treat my query as the same literal free text,
so that identical query strings return consistent results on both commands
regardless of FTS5 operator characters.

## Context And Value

`mdsearch search "foo AND bar"` submits `foo AND bar` verbatim into the FTS5
`MATCH` clause, so `AND` is interpreted as a boolean operator and unquoted
tokens, `prefix*`, and quotes act as FTS5 syntax — which can also surface a
fragile, message-text-dependent `InvalidQuery` error. `mdsearch hybrid "foo AND
bar"` routes through the domain's free-text neutralizer, which wraps each
whitespace term in double quotes (escaping embedded quotes by doubling) and
AND-joins them, so the same string is treated as literal text that must all be
present.

The result is two different semantics for the identical query string depending
on which command is used — surprising to document and painful for agent-harness
callers that switch between commands, and it complicates ADR-004 golden-set
evaluation. The neutralization behavior already lives in the domain with unit
test coverage, so the asymmetry is an oversight in the lexical path, not a
missing capability.

This story makes literal free-text semantics the single source of truth for
both retrieval commands, keeps the `EmptyQuery` and `InvalidQuery` error
contracts intact (with deterministic classification), and re-baselines the
golden evaluation set so quality targets remain verified.

## Business Rules

- `search` and `hybrid` apply the same query-to-FTS5 mapping: whitespace-
  separated terms are AND-joined, each term is quoted, and embedded quotes are
  doubled. Typed operator characters (`AND`, `OR`, `NOT`, `*`, `^`, `~`,
  parentheses, hyphens, quotes) are treated as literal text, never as FTS5
  operators.
- A query that is empty or whitespace-only yields the existing `EmptyQuery`
  error on both commands; neither command fails for any other reason related to
  query content.
- The `InvalidQuery` error contract remains as defense-in-depth, but
  classification becomes deterministic: it must not depend on matching `"fts5"`
  in SQLite's human-readable error message. It is unreachable for normal user
  input.
- The change never alters the ranking algorithm, fusion parameters, or result
  shape of either command — only the interpretation of the query string.
- The ADR-004 evaluation harness (`cargo xtask eval`) re-runs after the change;
  if scores shift, the golden baseline is updated and the quality targets
  (Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75) are re-verified.
- Other surfaces are untouched: `--related`, `context`, `get`, and the JSON
  output shapes do not change.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A collection contains files with the literal tokens `foo` and `bar`, and no operator meaning is intended | I run `mdsearch search "foo AND bar"` | `AND` is a literal term; results require `foo` and `bar` to be present — identical semantics to `hybrid` |
| EX-002 | A collection has both lexical and semantic indexes built | I run `mdsearch search "borrowing rust"` and `mdsearch hybrid "borrowing rust"` | Both commands return the same ranked passages for the same query string |
| EX-003 | My query contains `*`, `"`, `-`, or `OR` characters | I run `search` or `hybrid` | The characters match literally; no syntax error and no operator behavior |
| EX-004 | My query is `""` or whitespace-only | I run `search` or `hybrid` | Both commands report a clear `EmptyQuery` error |
| EX-005 | The golden set is re-run after the change | I run `cargo xtask eval` | Scores are recorded; if they changed, the baseline is updated and ADR-004 targets still hold |

## Acceptance Criteria

- `search` and `hybrid` produce identical results for the identical query
  string against the same collection state.
- FTS5 operator characters in a query match literally on both commands and
  never produce a syntax error for normal input.
- `EmptyQuery` is preserved for empty or whitespace-only queries on both
  commands.
- `InvalidQuery` classification is deterministic and independent of engine
  message text (OBS-003), and stays a defense-in-depth path.
- A regression scenario covering an operator-character query is added to the
  `007-lexical-search` feature packet.
- `cargo xtask eval` re-runs green after the change; the golden baseline is
  updated if scores shifted, and ADR-004 targets remain met.

## Scope Boundaries

### In Scope

- Applying the shared free-text-to-FTS5 mapping to the lexical search command
  path (`crates/application/src/lexical_search.rs`).
- Deterministic classification of the `InvalidQuery` error path
  (`crates/adapters/store-sqlite`), independent of SQLite message text.
- Regression scenarios in the `007-lexical-search` and `011-hybrid-search`
  feature packets.
- Re-running and, if needed, re-baselining the ADR-004 evaluation fixtures.

### Out Of Scope

- Changing the hybrid search command, its ranking, or fusion behavior.
- Introducing a raw-FTS5 opt-in flag or any other query-mode switch.
- Changing CLI switches, output formats, or JSON shapes.
- Empty-query semantics on `context`, `get`, or any non-search command.
- Other TODO.md observations (OBS-002, OBS-004, ...).

## Dependencies

- `US-007` (EPIC-003) provides the lexical search command whose query handling
  this story changes.
- `US-011` (EPIC-004) provides the hybrid search command and the reference
  neutralization behavior this story generalizes.
- `ADR-004` and the `xtask` evaluation harness provide the golden set and the
  re-baselining mechanism.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Which deterministic mechanism classifies `InvalidQuery` (pre-validation vs. rusqlite error variant vs. SQLite extended result code)? | No | TBD | Deferred to requirements/design |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable