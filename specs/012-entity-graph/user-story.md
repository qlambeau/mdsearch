---
id: US-012
title: "Deterministic entity graph build and internal query layer"
type: user-story
status: approved
created: 2026-08-19
updated: 2026-08-19
owner: TBD
parent: PRD-001
epic: EPIC-005
feature: 012-entity-graph
related:
  - US-004
  - US-006
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want `mdsearch update` to deterministically build an entity graph for each
collection — files, tags, and aliases as nodes, with typed edges derived from
frontmatter (`related:`, `sources:`), inline `.md` links, and `tags:` — stored
in the same SQLite database,
so that retrieval can later surface links to related concepts with zero LLM or
network cost.

## Context And Value

EPIC-004 added lexical (`mdsearch search`) and semantic (`mdsearch embed` /
`mdsearch hybrid`) retrieval. Both are passage-level: they rank text snippets,
not the relationships between the knowledge those snippets describe. The PRD
(EPIC-005) calls for a third, entity-level index: a graph whose nodes are
entities (files, tags, aliases) and whose edges are the relationships between
them, so that a query or agent harness can see which concepts connect to which.

This story takes the deterministic route: the graph is derived from structure
that already exists in the collection — the frontmatter fields the ingestion
pipeline already parses and the inline `.md` links between files — with no LLM
extraction. This matches the PRD's local-first constraint, keeps the build fast
and reproducible, and satisfies the established "extraction is an ingestion-time
investment" pattern (wiki: `knowledge-graph.md`, `synthesis-knowledge-graph-relation-extraction-during-ingestion.md`). The `async_graphql` query layer stays internal (ADR-001, DEC-009); the public related-links switch belongs to EPIC-006.

Both personas benefit: the developer-curator gets a connected map of their vault
that answers "what is related to this concept" structurally, and the harness can
later expand retrieved passages through their graph neighborhood for grounded
context.

## Business Rules

- Entity-graph building folds into `mdsearch update`: updating a collection
  rebuilds its graph alongside the lexical index, in the same database file.
- The graph is rebuilt deterministically per collection on every update: the
  collection's nodes and edges are fully replaced from the collection's current
  file set, so nodes or edges left over from files that were deleted or renamed
  disappear automatically.
- A collection with no files, or with files that yield no nodes or edges,
  contributes an empty graph; it is still a valid state, not an error.
- **Nodes:**
  - One node per file in the collection, typed `file`, identified by the stable
    file ID the ingestion pipeline assigns.
  - One node per distinct tag, typed `tag`, derived from the frontmatter `tags:`
    field.
  - One node per distinct alias, typed `alias`, derived from the frontmatter
    `aliases:` field.
- **Edges:**
  - `LINKS_TO` — from a file node to the file node of a target of an inline
    relative `.md` link inside that file.
  - `TAGGED_WITH` — from a file node to each of its tag nodes.
  - `ALIAS_OF` — from a file node to each of its alias nodes.
  - `RELATED_TO` — from a file node to each file named in its frontmatter
    `related:` field.
  - `HAS_SOURCE` — from a file node to each file named in its frontmatter
    `sources:` field.
- `related:` and `sources:` references that do not resolve to a known file in
  the collection are recorded as edges to the referenced name only if that name
  exists as a file; unresolved references are skipped, not errors.
- Edges are directional and typed; traversal is hop-limited in the query layer
  with a cycle guard.
- The build is idempotent: re-running `update` with unchanged files produces the
  same graph, with no duplicate nodes or edges.
- The internal query layer exposes at least: lookup of a node by ID, neighbor
  expansion along optional relation filters, and hop-limited traversal. It is
  the layer EPIC-006 will later drive with the public related-links switch.
- A debug CLI `mdsearch graph neighbors ID` inspects the graph from the shell
  for verification and development; it is not the public related-links surface.
- The build must never require network access or an LLM; it is fully local and
  deterministic.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `Notes` has two files where `a.md` links to `b.md` and both tag themselves `rust` | I run `mdsearch update` | The graph has file nodes `a` and `b`, a tag node `rust`, a `LINKS_TO` edge `a→b`, and `TAGGED_WITH` edges from both files to `rust` |
| EX-002 | `Notes` has a file with frontmatter `aliases: [mt, my]` | I run `mdsearch update` | The graph has two alias nodes and `ALIAS_OF` edges from the file to each alias |
| EX-003 | `Notes` has a file with frontmatter `related: [b]` and `sources: [c]` where both exist | I run `mdsearch update` | The graph has `RELATED_TO` and `HAS_SOURCE` edges from the file to `b` and `c` |
| EX-004 | `Notes` has a file whose `related:` names a file that does not exist in the collection | I run `mdsearch update` | The unresolved reference is skipped; the update succeeds and the graph contains no edge for it |
| EX-005 | `Notes` previously built a graph, then `c.md` is deleted from the filesystem | I run `mdsearch update` | The rebuild drops `c`'s node and every edge touching it; no stale entries remain |
| EX-006 | A collection has no files | I run `mdsearch update` | The update succeeds and the collection's graph is empty |
| EX-007 | A graph exists for `Notes` | I run `mdsearch graph neighbors a.md` | The CLI lists `a.md`'s neighbors with their relation types and depths |
| EX-008 | `Notes` has a file whose alias `mt` duplicates an existing tag `mt` | I run `mdsearch update` | The alias node and the tag node are distinct node rows; they are not merged |

## Acceptance Criteria

- `mdsearch update` builds the entity graph for each updated collection in the
  same SQLite database, alongside the lexical index.
- The graph contains file, tag, and alias nodes and typed, directional edges
  (`LINKS_TO`, `TAGGED_WITH`, `ALIAS_OF`, `RELATED_TO`, `HAS_SOURCE`) derived
  deterministically from file content and frontmatter.
- The build is a full deterministic rebuild: stale nodes and edges from removed
  files disappear, and re-running on unchanged files yields the same graph with
  no duplicates.
- Unresolved `related:`/`sources:` references are skipped without failing the
  update.
- The internal query layer supports node lookup, filtered neighbor expansion,
  and hop-limited traversal with a cycle guard.
- `mdsearch graph neighbors ID` inspects the graph from the shell for
  verification.
- The build runs fully offline with no LLM dependency.
- Empty collections build an empty graph without error.

## Scope Boundaries

### In Scope

- Entity-graph build folded into `mdsearch update`.
- New `nodes`, `edges`, and per-collection graph-state storage in the embedded
  database (schema version 6), mirroring the existing state-table pattern.
- Deterministic extraction of file, tag, and alias nodes and the five typed edge
  kinds listed above.
- Internal `async_graphql` query layer for node lookup, neighbor expansion, and
  hop-limited traversal.
- Debug CLI `mdsearch graph neighbors ID` for shell inspection.
- Tests covering the build, the rebuild, and the query layer.

### Out Of Scope

- LLM-based claim/relation extraction (`DEPENDS_ON`-style typed claims).
- The public related-links retrieval switch on `search`/`hybrid` (EPIC-006).
- Changes to the lexical or semantic index behavior or output.
- Entity resolution or deduplication beyond the deterministic node types above.
- Graph commands beyond the debug `mdsearch graph neighbors` inspection.

## Dependencies

- `US-004` provides the file store and the frontmatter parsing (title, tags,
  aliases, summary) that nodes and edges are derived from.
- `US-006` provides the stable file IDs and the passage/position identity that
  file nodes and inline link targets resolve against.
- `ADR-001` / `DEC-009` select SQLite as the single-file store and
  `async_graphql` as the internal in-process graph query layer.
- `ADR-005` establishes the full-rebuild-on-update precedent that this story
  follows for the graph.
- `DB-001` and `TABLE-001`..`TABLE-009` define the database schema that this
  story extends with the graph tables.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Should alias nodes that collide with tag node names be merged, or kept distinct? | No | TBD | Kept distinct (EX-008); revisit if a future slice needs entity resolution |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable