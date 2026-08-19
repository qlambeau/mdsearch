---
id: US-013
title: "Recover context from the entity graph"
type: user-story
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: PRD-001
epic: EPIC-007
feature: 013-context-recovery
related:
  - US-007
  - US-008
  - US-011
  - US-012
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want to recover file-to-file related context from the entity graph — via a
`--related` switch on `search`/`hybrid` and a dedicated `mdsearch context`
GraphQL command —
so that I can fill an LLM context window with grounded, related knowledge at
zero LLM/network cost.

## Context And Value

EPIC-005 builds a deterministic entity graph per collection (file, tag, and
alias nodes with typed, directional edges) and an internal in-process query
layer, but that graph is not yet reachable from retrieval. The harness and the
curator currently get ranked passages without the surrounding context that makes
them actionable: which files does a result relate to, and what does the graph
say about a concept?

This story exposes the graph's value on both sides of the product. `--related`
on `search`/`hybrid` enriches each result with its file-to-file links so a human
or harness can follow the connection to related knowledge immediately. A
dedicated `mdsearch context` command lets a harness issue precise in-process
GraphQL queries against the entity graph to recover grounded context for prompt
filling. Both reuse the EPIC-005 graph and query layer with no new storage, no
network, and no LLM, honoring the PRD's local-first, retrieval-only boundary.

## Business Rules

- `--related` on `search`/`hybrid` lists, for each returned result, that result
  file's file-to-file related links drawn from the closed edge set
  `LINKS_TO`, `RELATED_TO`, and `HAS_SOURCE`. Tag and alias nodes are never
  listed; related links are deduplicated.
- In human-readable output, each related file renders on its own line under its
  result as `related: <path> (<RELATION>)`.
- In machine-readable JSON output, each result gains a `related` field carrying
  the related file paths and their relation types; the rest of the JSON shape is
  unchanged.
- A result whose file has no file-to-file related links contributes no related
  output and does not fail the command.
- `--related` inherits the command's existing `--collection` and `--database`
  selection; it does not change the ranked results themselves, only the per-result
  context.
- `mdsearch context '<graphql query>'` accepts a positional GraphQL query string
  and executes it against the existing in-process entity-graph schema (node
  lookup, neighbor expansion with an optional relation filter, and hop-limited
  traversal). The output is JSON.
- `mdsearch context` requires `--collection NAME` and honors `--database PATH`.
- Both surfaces are strictly read-only and offline: they never mutate the graph,
  the database, or stored files, never perform network access, and never call an
  LLM. GraphQL stays in-process; no GraphQL server is exposed.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `search rust` returns `a.md`, which links to `b.md` and is tagged `rust` | I run `mdsearch search rust --related` | The output lists `related: b.md (LINKS_TO)` under `a.md`; the tag node `rust` is not listed |
| EX-002 | `search rust` returns `a.md`, which links to `b.md` | I run `mdsearch search rust --related --json` | The JSON result for `a.md` includes a `related` field with `b.md` and its relation |
| EX-003 | Collection `Notes` has a graph where `a.md` links to `b.md` | I run `mdsearch context '{ neighbors(collection: "Notes", kind: "file", key: "a.md", maxHops: 2) { key relation depth } }' --collection Notes` | The command prints the neighbors of `a.md` as JSON |
| EX-004 | The graph has no node for `zzz.md` | I query that node via `mdsearch context` | The command reports a clear error for the unknown node |
| EX-005 | The selected database does not exist | I run either command | The command reports the database does not exist and creates no file |

## Acceptance Criteria

- `--related` enriches `search` and `hybrid` output — human and JSON — with
  deduplicated file-to-file related links only, per result, without altering the
  ranked results.
- `mdsearch context '<graphql query>'` executes the query over the entity graph,
  requires `--collection`, honors `--database`, and prints JSON.
- Missing database, unknown collection, unknown node, and malformed GraphQL
  produce clear errors; a missing database never creates a file.
- Both commands run fully offline with no LLM or network access and never modify
  stored state.

## Scope Boundaries

### In Scope

- `--related` switch on `search` and `hybrid` (human and JSON output).
- Dedicated `mdsearch context '<graphql query>'` command with `--collection` and
  `--database`.
- Error handling for missing database, unknown collection, unknown node, and
  malformed GraphQL queries.
- Tests covering both surfaces and their error paths.

### Out Of Scope

- LLM-based claim or relation extraction.
- External services of any kind.
- Any mutation of the graph, database, or stored files.
- Exposing a GraphQL server or any network endpoint.
- Changing `search`/`hybrid` ranked results; `--related` only adds context.
- Applying `--related` to `get` or other commands.

## Dependencies

- `US-012` (EPIC-005) provides the entity graph, `SqliteGraphStore`, and the
  in-process `async_graphql` query layer this story drives.
- `US-007` and `US-011` provide the `search` and `hybrid` commands whose output
  `--related` enriches.
- `US-008` provides the machine-readable JSON output conventions extended by the
  `related` field.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Should the `related` field in JSON expose relation types as strings or as a nested object? | No | TBD | Deferred to requirements/design |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
