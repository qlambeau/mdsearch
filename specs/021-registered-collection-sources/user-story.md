---
id: US-021
title: "Remember collection sources and reconcile them on update"
type: user-story
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the proposed US-021 story and approved source listing in human/JSON output in chat on 2026-09-30"
parent: PRD-001
epic: EPIC-002
feature: 021-registered-collection-sources
depends_on:
  - US-005
  - US-006
requires: []
blockers: []
related:
  - US-012
  - ADR-005
---

# User Story

## Story Card

As a developer-curator,
I want each collection to remember its Markdown file and directory sources,
so that an update can discover new files and keep the collection current without
requiring me to repeat source paths.

## Context And Value

Today, a collection stores indexed files but not the paths they came from.
Updates therefore depend on paths being supplied again, and `update --all`
cannot discover files added since the last update. Persisting each collection's
sources makes explicit updates complete and repeatable while preserving the
existing local database as the source of truth for indexed content.

## Business Rules

- `mdsearch collection create NAME PATH...` registers one or more Markdown
  files or directories as sources without indexing their contents.
- `mdsearch collection configure NAME --sources PATH...` replaces the
  collection's registered source set.
- `mdsearch collection list` shows each collection's registered sources in
  human-readable output; `--json` provides the same collection/source data for
  scripts.
- Source paths are stored in canonical form. A registered file is considered
  directly; a registered directory is scanned recursively for Markdown files.
- Updating a collection scans its registered sources and reconciles stored
  files: newly discovered files are added, changed files are refreshed, and
  files no longer present in any registered source are removed.
- A Markdown file discovered beneath overlapping sources is processed once.
- When a source is removed, files found only through that source are removed on
  the next update; files still found through another registered source remain.
- A directory source that becomes inaccessible aborts that collection's update
  and leaves its previously stored content and indexes unchanged.
- `--skip-unreadable` skips an unreadable individual file and reports it. If
  that path already has stored content, the prior content remains stored.
- A successful update commits the collection's file reconciliation and its
  lexical and graph indexes together. If any of those builds fails, the prior
  files and indexes remain available unchanged.
- `update --all` attempts every collection independently, reports each
  collection's outcome, continues after a collection fails, and exits with a
  failure status if any collection failed.
- Collections migrated from databases without registered sources remain
  searchable. Their sources are not inferred from stored file paths; an update
  fails with guidance to register sources first.
- Indexing remains explicit. Registering or changing sources alone does not
  ingest files or build indexes.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A new collection and a Markdown directory | I create the collection with that directory as a source | The source is registered; no files are indexed until update |
| EX-002 | A registered directory with `a.md`, then a new `b.md` is added | I update the collection | Both files are stored and indexed, including the newly discovered file |
| EX-003 | A file is reachable through two registered directories | I update the collection | The file is stored and indexed once |
| EX-004 | A registered source is removed and one of its files is absent from every remaining source | I update the collection | That stored file and its index entries are removed |
| EX-005 | A registered directory cannot be read | I update the collection | The update fails and the previous stored content and indexes remain unchanged |
| EX-006 | An individual source file is unreadable and already has stored content | I update with `--skip-unreadable` | The file is reported as skipped and its previously stored content remains |
| EX-007 | A migrated collection has stored files but no registered sources | I update the collection | The update fails with guidance to register sources; stored content remains searchable |
| EX-008 | Two collections are targeted and one has an inaccessible source | I run `update --all` | The other collection is updated, both outcomes are reported, and the command exits unsuccessfully |
| EX-009 | An index build fails during an update | I update the collection | No file or lexical/graph index changes from that update are committed |
| EX-010 | A collection has registered sources | I list collections in human or JSON format | The output includes the collection's canonical source paths |

## Acceptance Criteria

- Creating or configuring a collection persists its canonical source paths without indexing files.
- Collection listing shows each collection's canonical source paths in human-readable and JSON output.
- An update discovers new Markdown files and reconciles additions, edits, and deletions from all registered sources.
- Overlapping sources do not cause a file to be processed more than once.
- Removing a source removes its exclusively sourced files on the next successful update while preserving files found by remaining sources.
- An inaccessible source directory aborts its collection update without changing its previously committed content or indexes.
- `--skip-unreadable` preserves already stored content for skipped individual files.
- File content and lexical/graph indexes commit or roll back together for each collection.
- `update --all` continues after collection failures, reports individual outcomes, and returns failure if any collection fails.
- Migrated collections without sources remain searchable and receive actionable guidance when update is attempted; no source paths are guessed.
- Source registration or configuration alone does not index content.

## Scope Boundaries

### In Scope

- Registering and replacing file/directory sources for a collection.
- Displaying registered sources through collection listing for both human and JSON consumers.
- Recursive Markdown discovery, canonical-path deduplication, and source-based reconciliation.
- Atomic per-collection content, lexical-index, and graph-index updates.
- Per-collection failure reporting for update-all.
- Safe behavior for legacy collections with no source metadata.

### Out Of Scope

- Semantic-index enablement or semantic rebuilds during update; those belong to a later slice.
- Database-wide model selection and rebuilding semantic indexes.
- The final breaking command grammar beyond the source/update behavior required here.
- Background file watching or automatic indexing.
- Guessing registered roots from already stored file paths.

## Dependencies

- `US-005` provides collection file reconciliation and unreadable-file behavior.
- `US-006` provides the lexical index built during update.
- `US-012` provides the collection graph built during update.
- `ADR-005` establishes full lexical-index rebuild and atomic update behavior.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | None. | No | Quentin | Resolved with user confirmation: removing a source deletes files found only through that source on the next update. |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
