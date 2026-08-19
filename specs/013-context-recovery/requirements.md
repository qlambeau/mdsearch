---
id: REQ-013
title: "Context recovery from the entity graph requirements"
type: feature-requirements
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: US-013
related:
  - US-007
  - US-008
  - US-011
  - US-012
  - PRD-001
---

# Requirements

## Purpose And Actors

### Purpose

Expose the EPIC-005 entity graph through retrieval. A `--related` switch on
`search`/`hybrid` enriches each ranked result with its file-to-file related
links, and a dedicated `mdsearch context '<graphql query>'` command executes
in-process GraphQL queries over the entity graph to recover grounded context for
LLM prompt filling. Both surfaces are local, read-only, and require no LLM or
network access.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- No network service or LLM is involved; both surfaces read the entity graph
  already built by `mdsearch update`.

## Preconditions

- The user invokes `mdsearch search`/`hybrid` with `--related`, or `mdsearch
  context '<query>'`.
- The database exists at the default path or the supplied `--database PATH`.
- The collection exists; for `--related` the collection's entity graph was
  built by a prior successful `mdsearch update`.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Search/hybrid with related context | Query, optional `--collection`, optional `--database`, `--related`, optional `--json` | Ranked results unchanged, each enriched with file-to-file related links in human or JSON form | Database exists; collection exists (when `--collection` given); graph read-only |
| Context recovery | GraphQL query string (positional), `--collection NAME` (required), optional `--database PATH` | JSON result of the GraphQL query | Query is valid GraphQL over the entity-graph schema; database and collection exist |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `--related` on `search`/`hybrid` shall enrich each returned result with that result file's file-to-file related links drawn only from `LINKS_TO`, `RELATED_TO`, and `HAS_SOURCE`; tag and alias nodes are never listed and links are deduplicated. | Must | US-013; Scenario: --related lists file-to-file related links in human output; omits tag and alias nodes; works on hybrid search |
| FR-002 | In human output, `--related` shall render one line per related file as `related: <path> (<RELATION>)`; a result with no file-to-file related links contributes no related line and the command still succeeds. | Must | US-013; Scenario: --related lists file-to-file related links in human output; omits tag and alias nodes |
| FR-003 | In JSON output, `--related` shall add a `related` field per result carrying the related file paths and their relation types, without changing any other field of the JSON shape. | Must | US-013; Scenario: --related adds a related field to JSON output |
| FR-004 | `--related` shall not change the ranked results themselves; it only adds per-result context. | Must | US-013; Scenario: --related does not change ranked results |
| FR-005 | `--related` shall inherit the invoking command's `--collection` and `--database` selection. | Must | US-013; Scenario: --related lists file-to-file related links in human output; works on hybrid search |
| FR-006 | `mdsearch context '<query>'` shall accept a positional GraphQL query string, execute it against the in-process entity-graph schema (node lookup, neighbor expansion with an optional relation filter, and hop-limited traversal), and print the result as JSON. | Must | US-013; Scenario: mdsearch context returns neighbors as JSON; supports node lookup |
| FR-007 | `mdsearch context` shall require `--collection NAME` and honor `--database PATH`. | Must | US-013; Scenario: mdsearch context requires a collection |
| FR-008 | `mdsearch context` shall report a clear error when the queried node does not exist. | Must | US-013; Scenario: mdsearch context reports an unknown node |
| FR-009 | `mdsearch context` shall report a clear error when the supplied query is not valid GraphQL. | Must | US-013; Scenario: mdsearch context rejects a malformed query |
| FR-010 | Both `--related` and `mdsearch context` shall fail with a clear error when the selected database does not exist, and must not create a database file. | Must | US-013; Scenario: Context recovery reports a missing database without creating one |
| FR-011 | Both surfaces shall be strictly read-only and offline: they never mutate the graph, database, or stored files, never perform network access, never call an LLM, and expose no GraphQL server. | Must | US-013; Business rules |
| FR-012 | A result whose file has no file-to-file related links (including when the graph or node is absent) shall produce no related output and the command succeeds. | Must | US-013; Scenario: --related omits tag and alias nodes |

## Postconditions And Invariants

- After a successful invocation, the entity graph, the database, and the stored
  files are unchanged; both surfaces are read-only.
- `--related` output is deterministic for a given graph and result set, and each
  result lists each related file at most once.
- `mdsearch context` output is the JSON-encoded result of the submitted GraphQL
  query over the entity-graph schema.
- `--related` never lists tag or alias nodes; the related set is always a subset
  of the file-to-file edges (`LINKS_TO`, `RELATED_TO`, `HAS_SOURCE`).

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Database does not exist | Fail without creating a file | Clear error |
| Collection does not exist (with `--collection`) | Fail | Clear error |
| `mdsearch context` without `--collection` | Fail | Clear error |
| Unknown node in `mdsearch context` query | Fail | Clear error |
| Malformed GraphQL query in `mdsearch context` | Fail | Clear error |
| Result file has no file-to-file links | No related output for that result | Command succeeds; no related line/field |
| Graph missing or never built for the collection | No related output (empty related set) | Command succeeds |

## Quality Requirements

- The build and query surfaces operate fully offline by default and never require
  a network or LLM service at runtime.
- Both surfaces are read-only and deterministic for identical inputs.
- Context recovery is best-effort with no explicit latency budget; it is bounded
  by the graph size and the hop limit of the submitted query (DEC-012).

## Dependencies And Deferred Decisions

- `US-012` (EPIC-005) provides the entity graph, `SqliteGraphStore`, and the
  in-process `async_graphql` query layer that both surfaces drive.
- `US-007` and `US-011` provide the `search` and `hybrid` commands whose output
  `--related` enriches.
- `US-008` provides the machine-readable JSON output conventions extended by the
  `related` field.
- The exact serialization of the JSON `related` field (relation type as a string
  vs a nested object) is deferred to design (OQ-001).

## Traceability

- Source story: `US-013` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
