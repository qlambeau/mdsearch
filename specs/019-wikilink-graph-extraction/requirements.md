---
id: REQ-019
title: "Wikilink graph extraction requirements"
type: feature-requirements
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-019
related:
  - US-012
  - US-013
  - REQ-012
  - REQ-013
---

# Requirements

## Purpose And Actors

### Purpose

Make knowledge graph extraction recognize Obsidian-style wikilinks
(`[[note]]`, `[[note|label]]`, `[[path/note#heading]]`) as `LINKS_TO` edges
alongside standard markdown links, resolving targets against known collection
files case-insensitively and skipping unresolved, ambiguous, and self-referential
links. The feature completes EPIC-013 (OBS-009).

### Actors And External Systems

- Developer-curator whose vault uses wikilink syntax, running
  `mdsearch collection update`.
- The entity graph consumer (`--related` on `search`/`hybrid`, `graph
  neighbors`, `mdsearch context`) — benefits without changes.
- The local collection database: existing `nodes`/`edges` tables, no schema
  change.

## Preconditions

- The user runs `mdsearch collection update` (or `update --all`) on a
  collection whose files may contain wikilinks.
- The existing graph extraction pipeline from `REQ-012`/`REQ-013` — frontmatter
  entities, inline markdown links, graph rebuild and fingerprinting — remains
  in force.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Update a collection | `update NAME PATH...` (files with wikilink content) | `LINKS_TO` edges in the graph for every resolved wikilink | The wikilink target resolves to exactly one known file, case-insensitively, and is not the source file |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | Graph extraction shall recognize `[[target]]`, `[[target\|label]]`, `[[path/target#heading]]`, and `[[target#heading\|label]]` and create one `LINKS_TO` edge from the source file to the resolved target file. | Must | US-019; A plain wikilink creates a LINKS_TO edge; A wikilink resolves to a file through a path |
| FR-002 | The piped label of `[[target\|label]]` shall be ignored for graph purposes: exactly one `LINKS_TO` edge is created and no alias node or `ALIAS_OF` edge is produced. | Must | US-019; A piped wikilink targets the file, ignoring the label |
| FR-003 | A header fragment (`#heading`) in a wikilink shall be stripped before resolution; the edge targets the file. | Must | US-019; A header fragment is stripped before resolution |
| FR-004 | A bare self-anchor (`[[#heading]]`) shall produce no cross-file edge. | Must | US-019; A bare self-anchor produces no edge |
| FR-005 | Wikilink target resolution shall be case-insensitive against the known collection files, resolving only when exactly one known file matches. When more than one known file matches case-insensitively (e.g. `Note.md` and `note.md`), the target shall be treated as unresolved and skipped. | Must | US-019; Wikilink resolution is case-insensitive; An ambiguous case-only match is skipped |
| FR-006 | An unresolved wikilink target shall produce no edge and no error. | Must | US-019; An unresolved wikilink produces no edge |
| FR-007 | A resolved wikilink whose target is the source file itself shall be skipped: no self-edges are created. | Must | US-019; A self-link produces no edge |
| FR-008 | Standard markdown link extraction (`[label](target.md)`) shall remain unchanged; both markdown links and wikilinks in the same file are extracted. | Must | US-019; Markdown links and wikilinks both produce edges |
| FR-009 | A wikilink whose target is empty, a bare fragment (`[[#...]]` per FR-004), or an `http(s)` URL shall produce no edge. | Must | US-019 (scope; examples) |
| FR-010 | Scenario coverage for the wikilink forms, case-insensitivity, ambiguity, unresolved targets, and self-links shall be added to the `012-entity-graph` feature packet. | Must | US-019 (acceptance criteria) |

## Postconditions And Invariants

- Every extracted wikilink edge is a `LINKS_TO` edge between two distinct file
  nodes, both of which exist as files in the collection.
- Graph extraction remains deterministic: identical content produces identical
  edges, including under case-insensitive matching.
- The `nodes`/`edges` schema is unchanged; a graph rebuild regenerates the
  wikilink edges from file content.
- Standard markdown-link behavior is byte-for-byte unchanged.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Target matches no known file | Skipped, no edge, no error | Graph unchanged for that link |
| Target matches two files case-insensitively | Treated as unresolved, skipped | No ambiguous edge |
| Target is the source file | Skipped (self-edge) | No self-loop |
| `[[#heading]]` | No cross-file edge | No edge |
| `[[http://...]]` or `[[https://...]]` | No edge | No edge |
| Target names a non-markdown known file | No edge (markdown files only, matching current behavior) | No edge |

## Quality Requirements

- Deterministic and pure: the extraction lives in the domain layer with no I/O,
  clock, or randomness (R-SEP-02).
- Additive: existing graph behavior and output shapes are unchanged.
- Case-insensitive resolution cost is bounded: a single pass over known file
  names per distinct wikilink target.
- No schema, store, adapter, or CLI changes.

## Dependencies And Deferred Decisions

- The domain graph extraction (`crates/domain/src/graph.rs`) owns the change;
  `REQ-012`/`REQ-013` contracts stay in force (R-SDD-05).
- The `012-entity-graph` packet's scenarios and tests are extended for the new
  behavior (FR-010).
- Code-fence-aware extraction remains a separate observation (OBS-010) and is
  explicitly out of scope.

## Traceability

- Source story: `US-019` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-013, DEC-018)