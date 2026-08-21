---
id: US-019
title: "Wikilink graph extraction"
type: user-story
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: PRD-001
epic: EPIC-013
feature: 019-wikilink-graph-extraction
related:
  - US-012
  - US-013
---

# User Story

## Story Card

As a developer-curator whose markdown vault uses Obsidian-style wikilinks,
I want `mdsearch update` to extract `[[note]]`, `[[note|label]]`, and
`[[path/note#heading]]` references as `LINKS_TO` graph edges,
so that wikilink-based vaults produce connected entity graphs and
`--related`/`graph neighbors` surface real document relationships.

## Context And Value

Knowledge graph extraction parses only standard Markdown links of the form
`[label](target.md)` (OBS-009). The primary target domain for `mdsearch` is
developer markdown knowledge vaults (Obsidian, Logseq, Foam, Dendron, QMD),
where intra-vault cross-references are predominantly written as wikilinks:
`[[target]]`, `[[target|label]]`, `[[path/target#heading]]`. None of these are
recognized today, so graphs extracted from wikilink-based vaults remain
largely disconnected — zero `LINKS_TO` edges — and `--related` and
`graph neighbors` miss document relationships.

This story adds wikilink extraction alongside the existing standard-markdown
link extraction, resolves targets against known collection files
(case-insensitively, Obsidian-style), and skips unresolved targets and
self-edges.

## Business Rules

- Wikilinks of the forms `[[target]]`, `[[target|label]]`,
  `[[path/target#heading]]`, and `[[target#heading|label]]` are recognized
  during graph extraction.
- A header fragment (`#heading`) is stripped before resolution; the edge
  targets the file.
- A wikilink with a piped label (`[[target|label]]`) creates one `LINKS_TO`
  edge to the target file; the label is display text only and does not create
  graph nodes.
- A bare self-anchor (`[[#heading]]`) produces no cross-file edge.
- Resolution is case-insensitive against the known collection files: `[[Note]]`
  resolves to `note.md`. When more than one known file matches
  case-insensitively (e.g. `Note.md` and `note.md`), the target is treated as
  unresolved and skipped.
- A resolved link whose target is the source file itself is skipped (no
  self-edges).
- Unresolved wikilink targets produce no edge, matching the existing
  markdown-link behavior.
- Wikilink extraction is additive: standard markdown links `[label](target.md)`
  keep their current behavior and are still extracted.
- Extraction scans raw content like the existing link extraction; code-fence
  awareness is out of scope (OBS-010).

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A vault with `borrowing.md` | I run `mdsearch collection update` on a file containing `[[borrowing]]` | A `LINKS_TO` edge from the source file to `borrowing.md` |
| EX-002 | A vault with `notes/borrowing.md` | I run update on a file containing `[[notes/borrowing|Borrowing Rules]]` | A `LINKS_TO` edge to `notes/borrowing.md`; no alias node |
| EX-003 | A vault with `borrowing.md` | I run update on a file containing `[[borrowing#Lifetimes]]` | A `LINKS_TO` edge to `borrowing.md` (fragment stripped) |
| EX-004 | A vault with `note.md` | I run update on a file containing `[[Note]]` | A `LINKS_TO` edge to `note.md` (case-insensitive match) |
| EX-005 | No known file matches | I run update on a file containing `[[missing]]` | No edge is created |
| EX-006 | `note.md` links to itself | I run update on `note.md` containing `[[note]]` | No self-edge is created |
| EX-007 | A vault with both `Note.md` and `note.md` | I run update on a file containing `[[Note]]` | The target is ambiguous and skipped |
| EX-008 | A file with `[label](target.md)` and `[[target]]` | I run update | Both the markdown link and the wikilink produce edges |

## Acceptance Criteria

- `[[target]]`, `[[target|label]]`, `[[path/target#heading]]`, and
  `[[target#heading|label]]` all produce `LINKS_TO` edges to the resolved file.
- `[[#heading]]` produces no edge.
- Case-insensitive resolution works; ambiguous case-only matches are skipped.
- Self-links produce no edges.
- Unresolved targets produce no edges and no errors.
- Standard markdown link extraction is unchanged.
- Scenario coverage is added to the `012-entity-graph` feature packet.

## Scope Boundaries

### In Scope

- Wikilink recognition in the domain graph extraction
  (`crates/domain/src/graph.rs`).
- Case-insensitive resolution against known files with ambiguity skipping.
- Self-edge skipping for resolved links.
- Scenario and unit-test coverage in `012-entity-graph` and this packet.

### Out Of Scope

- `Alias` nodes / `ALIAS_OF` edges from piped labels.
- Code-fence-aware extraction (OBS-010).
- Block references (`[[^block]]`) and embedded embeds (`![[note]]`).
- Case-insensitive resolution for standard markdown links.
- Changing `resolve_file`, frontmatter handling, or other graph features.
- Other TODO.md observations (OBS-004, OBS-005, ...).

## Dependencies

- `US-012` (EPIC-005) provides the entity-graph extraction whose link pass
  this story extends.
- `US-013` (EPIC-007) consumes the graph for `--related`/`context`; it benefits
  without changes.
- The domain layer owns the extraction; no adapter or store changes are
  required.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Does the store's graph rebuild (fingerprint-driven) automatically pick up the new edges without a schema change? | No | TBD | Resolved: edges live in the existing `edges` table; a rebuild regenerates them from content |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable