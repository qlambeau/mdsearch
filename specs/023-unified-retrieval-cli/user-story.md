---
id: US-023
title: "Unified retrieval and final CLI surface"
type: user-story
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the synthesized US-023 story in chat; status freshness compares indexes with stored collection content"
parent: PRD-001
epic: EPIC-006
feature: 023-unified-retrieval-cli
depends_on:
  - US-008
  - US-009
  - US-011
  - US-013
  - US-020
  - US-021
  - US-022
requires: []
blockers: []
related:
  - ADR-004
  - ADR-008
  - ADR-009
  - ADR-010
  - ADR-016
  - ADR-017
---

# User Story

## Story Card

As a developer-curator or coding agent harness,
I want consistent commands for searching, retrieving files, and inspecting
indexes and graphs,
so that I can retrieve grounded content without guessing command syntax or
collection scope.

## Context And Value

Registered sources and semantic configuration now support explicit updates of
all configured indexes. The remaining CLI surface still separates lexical and
hybrid retrieval, uses positional collection selection for file retrieval,
and duplicates collection selection inside graph queries. Results omit the
file IDs needed for reliable retrieval, while index inspection lacks complete
readiness and freshness reporting.

This final slice delivers the accepted breaking CLI redesign. It provides one
retrieval command, explicit file selection, collection-bound graph queries,
and inspection of configured indexes without changing retrieval ranking.

## Business Rules

- The final command surface is `collection create/configure/list/delete`,
  top-level `update`, `search`, `get`, `status`, `model list/set`, and
  `graph neighbors/query`. The transitional `collection add/destroy/update`,
  `embed`, `hybrid`, `index status`, and `context` commands are replaced.
- Collection creation requires a name and at least one source path; semantic
  indexing remains opt-in. Configuration can replace sources and/or change
  semantic enablement. Listing reports sources and enabled indexes. Deletion
  removes stored collection data and retains original files.
- `update` requires exactly one of `--collection NAME` and `--all`, retains
  explicit download and skip-unreadable switches, and supports JSON reports.
- `--database PATH` remains global and works at every command depth.
  Applicable commands accept `-c/--collection`; search accepts `-n/--limit`.
- `search QUERY...` accepts quoted queries or multiple query words, defaults
  to lexical mode, and supports `--mode lexical|hybrid`. The limit defaults to
  10 and must be between 1 and 100 inclusive.
- Search retains JSON output, related-file enrichment, and hybrid re-ranking
  behavior. `--no-rerank` is invalid in lexical mode. Hybrid prerequisite
  errors identify the missing prerequisite and provide a recovery command.
- Human search results identify the collection and file ID and explicitly
  report no matches. Existing JSON fields are preserved while adding `mode`
  and `file_id`.
- `get PATH_OR_NAME --collection NAME` retrieves exact stored content.
  `get --id ID --collection NAME` selects by explicit file ID. The two
  selectors are mutually exclusive; content is emitted without an added
  newline. Numeric file names remain path/name selectors unless `--id` is used.
- Graph inspection requires a collection. Neighbor lookup accepts explicit
  node kind, relation, and depth options; kind defaults to file and depth to
  one hop. Depth is between 1 and 255 inclusive.
- `graph query QUERY --collection NAME` binds every resolver to the selected
  collection. Public query fields no longer accept collection arguments.
- `status [--collection NAME] [--json]` reports index enablement, readiness,
  freshness, selected models, and last successful build times. Freshness
  compares indexes with stored collection content and does not scan sources
  for pending filesystem changes.
- All commands provide descriptive help and examples. Help and version use
  stdout and exit 0; argument errors exit 2; operational errors exit 1.
  HOME is resolved only when a required default path needs it.
- Documentation publishes an old-to-new command mapping and the complete
  replacement workflow. Offline operation and retrieval quality are preserved.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | Notes has registered sources | I run `update -c Notes` | Every configured index is refreshed with the existing atomic update behavior |
| EX-002 | Notes contains matching passages | I search for `borrowing rules` as multiple words | Lexical retrieval uses the same query as the quoted form and exposes file IDs |
| EX-003 | Notes has the prerequisites for hybrid retrieval | I search with `--mode hybrid -n 5` | Existing hybrid ranking returns at most five results |
| EX-004 | File 42 contains content without a final newline | I run `get --id 42 -c Notes` | Exactly the stored content is emitted |
| EX-005 | A stored file is named `42` | I run `get 42 -c Notes` | The name selector is used rather than interpreting 42 as an ID |
| EX-006 | Notes and Archive contain the same graph key | I run `graph query` with `-c Notes` | All queried nodes and neighbors come from Notes |
| EX-007 | Index fingerprints differ from stored content | I inspect status | The affected indexes are reported stale alongside their last successful builds |
| EX-008 | Source files changed after the last update but stored content did not | I inspect status | Freshness reflects stored content without scanning source files |

## Acceptance Criteria

- The replacement command grammar is implemented and obsolete commands are
  rejected with the documented argument-error convention.
- Unified search preserves lexical and hybrid retrieval behavior, validates
  modes and limits, and accepts quoted or multiple-word queries.
- Human and JSON results expose collection and explicit file identity;
  empty searches produce an explicit human message and valid empty JSON.
- File retrieval uses mutually exclusive name/path and ID selectors and emits
  exact stored bytes without an added newline.
- Graph neighbors use explicit options and collection selection; GraphQL
  queries cannot choose a different collection from the CLI-selected one.
- Human and JSON collection, update, and status output reflect the approved
  sources, enabled indexes, update outcomes, and stored-content freshness.
- Model management, atomic updates, migrations, global options, offline
  behavior, and established JSON fields remain supported.
- Help, subprocess diagnostics, and the migration mapping match the final
  grammar; repository quality and retrieval gates pass.

## Scope Boundaries

### In Scope

- Final breaking command grammar and old-to-new documentation.
- Unified search, explicit file selectors, provenance, and output contracts.
- Collection-bound graph querying and explicit neighbor inspection.
- Collection/index inspection and structured update reports.
- Readiness and freshness against stored content, model and build reporting.

### Out Of Scope

- Ranking, fusion, passage segmentation, or model algorithm changes.
- Scanning sources during status, file watching, or automatic indexing.
- New indexing policies, model catalogs, storage engines, or dependencies.
- Answer generation, graph mutation, hosted services, or new product surfaces.

## Dependencies

- US-008 and US-009 establish machine-readable results and stored-file retrieval.
- US-011 and US-013 establish hybrid ranking and contextual retrieval.
- US-020 establishes help, diagnostics, global options, and output streams.
- US-021 establishes registered sources and source-aware atomic updates.
- US-022 establishes semantic configuration and database-wide model management.

## Open Questions

None. Quentin confirmed that status compares indexes with stored content.

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
