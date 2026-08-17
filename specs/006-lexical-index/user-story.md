---
id: US-006
title: "Build the lexical index during collection update"
type: user-story
status: implemented
created: 2026-08-17
updated: 2026-08-17
owner: TBD
parent: PRD-001
epic: EPIC-003
feature: 006-lexical-index
related:
  - US-004
  - US-005
---

# User Story

## Story Card

As a developer-curator,
I want `collection update` to build and keep current a lexical (BM25-style)
index of my collection's content, and to check the index status,
so that the collection is ready for lexical search once a later slice adds the
search command.

## Context And Value

`US-005` reconciles a collection's stored files against the filesystem but never
indexes their content. This story makes `collection update` the explicit trigger
that builds a per-passage lexical index from the stored files, and adds
`mdsearch index status` so the build and its freshness are observable without a
search command yet. It is the first half of EPIC-003; the dedicated lexical
search command that returns ranked passages is a separate, later story.

## Business Rules

- `mdsearch collection update NAME PATH...` and `mdsearch collection update --all`
  preserve the reconciliation behavior from `US-005` (added, modified, deleted,
  unchanged) and then rebuild the lexical index for the affected collection(s)
  in the same transaction.
- `mdsearch collection add` alone never builds the index; the lexical index
  becomes current only after an update, consistent with the PRD constraint that
  indexing is driven by explicit update commands.
- The lexical index covers each stored file:
  - The body is segmented into passages by splitting on one or more blank lines;
    each paragraph is one indexed passage.
  - Each recognized frontmatter field (`title`, `tags`, `aliases`, `summary`)
    is indexed as its own passage.
  - A file without frontmatter is indexed body-only.
  - A file with malformed or unparseable frontmatter is indexed body-only, is
    reported, and does not fail the update.
  - An empty file with no paragraphs and no frontmatter fields contributes no
    passages but does not fail the update.
- The index build is atomic with the file changes: if the index build fails
  (for example, a storage error), the whole update fails, no file changes are
  committed, and re-running the update retries.
- `mdsearch index status` takes no collection name and reports, for every
  collection in the database, the lexical index state (`built` or `not built`),
  the stored file count, the indexed passage count, and the timestamp of the
  last index build.
  - `not built` means the collection has never had its index built by an update.
  - A collection with zero passages still shows `built` after a successful
    update.
  - If the selected database does not exist, the command fails and reports the
    database does not exist without creating a file.
  - A fresh database with no collections produces empty output.
- `--database PATH` overrides the default database path
  `~/.mdsearch/collections.db` for both commands.
- Exact human-readable wording of errors and stats output is not part of this
  story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A collection `Notes` stores `a.md`, never updated since adding | I run `mdsearch index status` | `Notes` shows state `not built` |
| EX-002 | `Notes` stores `a.md` with 3 body paragraphs, a `title`, and `tags` | I run `mdsearch collection update Notes a.md` | The update succeeds, the index is built, and `mdsearch index status` shows `built`, 1 file, 5 passages, and a build timestamp |
| EX-003 | `a.md` is edited to add 2 paragraphs | I run `mdsearch collection update Notes a.md` | The passage count grows to 7 and the build timestamp is refreshed |
| EX-004 | `b.md` is deleted from disk | I run `mdsearch collection update Notes b.md` | `b.md` is removed and its passages no longer appear in the passage count |
| EX-005 | `c.md` has malformed frontmatter | I run `mdsearch collection update Notes c.md` | `c.md` is indexed body-only, the malformed case is reported, and the update succeeds |
| EX-006 | `d.md` has no frontmatter and 2 paragraphs | I run `mdsearch collection update Notes d.md` | `d.md` contributes exactly 2 passages |
| EX-007 | `e.md` is empty | I run `mdsearch collection update Notes e.md` | The update succeeds, `e.md` contributes 0 passages, and the collection still shows `built` |
| EX-008 | A storage error occurs while building the index | I run `mdsearch collection update Notes a.md` | The whole update fails, the file changes are not committed, and the previous index state is unchanged |
| EX-009 | Two collections exist, one with edited files | I run `mdsearch collection update --all` | Both collections are reconciled and their indexes rebuilt; `mdsearch index status` reports both |
| EX-010 | No database exists at the selected path | I run `mdsearch index status --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- `collection update` rebuilds the lexical index for the updated collection(s)
  in the same transaction as the file reconciliation.
- `collection add` alone leaves the lexical index unbuilt.
- Body paragraphs and the `title`, `tags`, `aliases`, and `summary` frontmatter
  fields are each indexed as passages.
- Files without frontmatter are indexed body-only; files with malformed
  frontmatter are indexed body-only and reported without failing the update.
- An empty file contributes no passages without failing the update.
- A storage failure while building the index fails the whole update and commits
  no file changes.
- `mdsearch index status` reports per collection the built/not-built state, file
  count, passage count, and last-build timestamp.
- A missing database fails the `index status` command without creating a file.
- `--database PATH` selects the database used by both commands.

## Scope Boundaries

### In Scope

- Paragraph segmentation of file bodies by blank-line split.
- Lenient extraction of the `title`, `tags`, `aliases`, and `summary` frontmatter
  fields.
- Per-passage lexical index rows built during `collection update`.
- `mdsearch index status` reporting per-collection index statistics.
- Atomic failure semantics for the combined reconciliation-and-index operation.

### Out Of Scope

- The lexical search command and ranked passage retrieval (a later EPIC-003 story).
- Semantic or contextual indexing (EPIC-004 and EPIC-005).
- JSON output and diff-style positions (EPIC-006).
- Frontmatter fields beyond the four recognized ones.
- Frontmatter validation or schema enforcement beyond lenient extraction.

## Dependencies

- `US-005` provides the file reconciliation, content hashing, and the `files`
  table that the lexical index is built from.
- FTS5 in SQLite is the approved lexical-index mechanism (ADR-001).
- A YAML frontmatter parsing approach and its crate choice are design-time
  decisions for the next artifact.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | None. | No | TBD | Resolved |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable