---
id: REQ-012
title: "Deterministic entity graph build and internal query layer requirements"
type: feature-requirements
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: US-012
related:
  - US-004
  - US-006
  - ADR-001
  - ADR-005
  - DB-001
  - TABLE-001
  - TABLE-002
  - TABLE-003
---

# Requirements

## Purpose And Actors

### Purpose

Provide the entity-level contextual index of EPIC-005: `mdsearch update` builds a
deterministic entity graph for each collection — files, tags, and aliases as
nodes, with typed, directional edges derived from frontmatter (`related:`,
`sources:`, `tags:`, `aliases:`) and inline relative `.md` links — stored in the
same SQLite database as the lexical and semantic indexes. The slice also
provides an internal query layer for node lookup, filtered neighbor expansion,
and hop-limited traversal, plus a debug CLI (`mdsearch graph neighbors ID`) for
shell inspection. The public related-links retrieval switch is owned by EPIC-006.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- No network service or LLM is involved; the build is fully local and
  deterministic.

## Preconditions

- The user invokes `mdsearch update`, optionally with `--collection NAME` or
  `--database PATH`.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- The database exists.
- The collection exists and may store zero or more files; files were ingested
  via `mdsearch add` (US-004) with frontmatter parsed per the existing pipeline.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Update a collection | `mdsearch update`, optional `--collection NAME`, optional `--database PATH` | Collection's entity graph rebuilt in the database; success or a clear error | Collection exists (when `--collection` given); database exists; file set read deterministically |
| Rebuild after files changed | Changed file set (added, modified, deleted) | Stale nodes and edges removed; current nodes and edges present | Rebuild is full and deterministic per collection |
| Debug graph inspection | `mdsearch graph neighbors ID`, optional `--collection NAME`, optional `--database PATH` | Neighbor list with relation types and depths; empty output when the node has no neighbors | ID resolves to a node; database exists |
| Internal query layer | Node lookup, neighbor expansion with optional relation filter, hop-limited traversal | Typed query results for tests and later slices | Node exists; hop limit is finite; cycle guard active |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `mdsearch update` shall build the entity graph for each updated collection in the same SQLite database, alongside the lexical index, in the same run. | Must | US-012; Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges |
| FR-002 | The graph shall contain one node per stored file, typed `file`, identified by the stable indexing-assigned file ID. | Must | US-012; Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges |
| FR-003 | The graph shall contain one node per distinct frontmatter `tags:` value, typed `tag`. | Must | US-012; Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges |
| FR-004 | The graph shall contain one node per distinct frontmatter `aliases:` value, typed `alias`. | Must | US-012; Update creates alias nodes and ALIAS_OF edges |
| FR-005 | The graph shall contain a `LINKS_TO` edge from a file node to each file node targeted by an inline relative `.md` link within that file. | Must | US-012; Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges |
| FR-006 | The graph shall contain a `TAGGED_WITH` edge from each file node to each of its tag nodes. | Must | US-012; Update builds file and tag nodes with LINKS_TO and TAGGED_WITH edges |
| FR-007 | The graph shall contain an `ALIAS_OF` edge from each file node to each of its alias nodes. | Must | US-012; Update creates alias nodes and ALIAS_OF edges |
| FR-008 | The graph shall contain a `RELATED_TO` edge from a file node to each file named in its frontmatter `related:` field. | Must | US-012; Update creates RELATED_TO and HAS_SOURCE edges |
| FR-009 | The graph shall contain a `HAS_SOURCE` edge from a file node to each file named in its frontmatter `sources:` field. | Must | US-012; Update creates RELATED_TO and HAS_SOURCE edges |
| FR-010 | A `related:` or `sources:` reference that does not resolve to a known file in the collection shall be skipped, without failing the update and without creating an edge. | Must | US-012; Skip unresolved related and sources references |
| FR-011 | The graph build shall be a full deterministic rebuild: the collection's nodes and edges are replaced from its current file set, so nodes and edges left over from deleted or renamed files disappear. Re-running on an unchanged file set shall produce the same graph with no duplicate nodes or edges. | Must | US-012; Rebuild drops nodes and edges from deleted files; Rebuild on unchanged files produces the same graph |
| FR-012 | A collection with no stored files shall build an empty graph; the update shall succeed without error. | Must | US-012; An empty collection builds an empty graph |
| FR-013 | A tag node and an alias node with the same name shall remain distinct node rows; they shall not be merged. | Must | US-012; Alias and tag nodes with the same name remain distinct |
| FR-014 | `mdsearch graph neighbors ID` shall list the node's neighbors with their relation types and traversal depths from the shell. | Must | US-012; Inspect a node's neighbors with the debug CLI |
| FR-015 | The internal query layer shall support neighbor expansion from a node, optionally filtered by relation type. | Must | US-012; Query layer expands neighbors along a relation filter |
| FR-016 | The internal query layer shall support hop-limited traversal with a cycle guard so traversals terminate and do not revisit nodes. | Must | US-012; Query layer traversal stops at the hop limit |
| FR-017 | The graph build and query layer shall operate fully offline with no LLM dependency; no network access is performed at build or query time. | Must | US-012; Update builds the graph with no network or LLM dependency |
| FR-018 | The graph build shall be transactional per collection: either the collection's graph is fully replaced, or the previous graph remains unchanged on failure. | Must | US-012; Rebuild drops nodes and edges from deleted files |

## Postconditions And Invariants

- After a successful update, each updated collection's graph exactly reflects its current file set: every stored file has exactly one file node, every distinct tag and alias has exactly one node of its type, and every derived edge is present exactly once.
- The graph never contains a node or edge derived from a file that is no longer stored in the collection.
- Node identity is stable: a file node is identified by its stable file ID; a tag or alias node is identified by its exact normalized name and node type.
- The set of edge types is closed: `LINKS_TO`, `TAGGED_WITH`, `ALIAS_OF`, `RELATED_TO`, and `HAS_SOURCE`.
- The build never modifies the stored files, the collections, the lexical index, or the semantic index.
- The debug CLI and the internal query layer are read-only: they do not modify the graph or any other stored state.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Unresolved `related:`/`sources:` reference | Skip the reference, no edge created | Update succeeds; graph contains no edge for the reference |
| Empty collection | Build an empty graph | Update succeeds |
| Collection deleted or never created (with `--collection`) | Fail | Clear error |
| Database does not exist | Fail without creating a file | Output communicates the database does not exist |
| Node ID unknown in `mdsearch graph neighbors` | Fail | Clear error |
| Graph build failure mid-transaction | Roll back | Previous graph for the collection remains unchanged |
| Inline link target not among stored files | Skip the link, no edge created | Update succeeds; no `LINKS_TO` edge for the missing target |
| Alias name colliding with a tag name | Keep distinct rows | Both nodes exist; they are not merged |

## Quality Requirements

- The build is deterministic: the same file set always produces the same graph.
- The build is idempotent: re-running on unchanged files changes nothing.
- The build and query layer operate offline by default and never require a network or LLM service at runtime.
- Graph build cost is ingestion-time only; query-layer traversals of 1-3 hops over typical collections (100-5,000 files) must complete quickly enough for harness context-filling (PRD-001 soft latency target).

## Dependencies And Deferred Decisions

- The file store and frontmatter parsing from `US-004` provide the file set,
  file IDs, and `tags:`/`aliases:` values that nodes and edges are derived from.
- The stable file IDs from `US-006` identify file nodes and resolve inline link
  and `related:`/`sources:` targets.
- `ADR-001` / `DEC-009` select SQLite as the single-file store and `async_graphql`
  as the internal in-process graph query layer.
- `ADR-005` establishes the full-rebuild-on-update precedent that the graph build
  follows.
- The concrete `nodes`, `edges`, and `graph_state` DDL, the async-graphql schema,
  and the traversal implementation are deferred to design and recorded in new
  table specs (TABLE-010/011/012) and an ADR.
- The public related-links retrieval switch is owned by EPIC-006 and is out of
  scope for this contract.

## Traceability

- Source story: `US-012` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
