---
id: US-022
title: "Configure semantic indexing and models"
type: user-story
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the proposed US-022 story and clarified reranker-only model changes and legacy embed behavior in chat on 2026-09-30"
parent: PRD-001
epic: EPIC-004
feature: 022-semantic-configuration-model-management
depends_on:
  - US-010
  - US-011
  - US-015
  - US-021
requires: []
blockers: []
related:
  - ADR-006
  - ADR-007
  - ADR-010
---

# User Story

## Story Card

As a developer-curator,
I want to choose which collections maintain semantic indexes and manage the
database's models,
so that updates keep each enabled collection's indexes consistent and model
changes do not leave mixed or broken search state.

## Context And Value

The CLI can currently build semantic indexes through a separate `embed`
operation, while source-aware `update` rebuilds stored files, lexical passages,
and the entity graph. Semantic indexing is not configurable per collection, so
users cannot make one explicit update maintain every index they have chosen to
use. Embedding-model selection also needs a safe database-wide operation: all
enabled semantic indexes must use the selected model together.

This story makes semantic indexing opt-in for each collection, includes enabled
semantic indexes in source-aware updates, and provides explicit model
inspection and selection. A failed update or global model change preserves the
last working collection data and indexes.

## Business Rules

- New collections have semantic indexing disabled unless the user explicitly
  enables it.
- Collection configuration can enable or disable semantic indexing.
- Until the final command-grammar slice replaces it, the existing `embed`
  command shall require semantic indexing to be enabled for each targeted
  collection. It shall not enable semantic indexing implicitly.
- When semantic indexing is disabled, the next successful update removes that
  collection's stored vectors and semantic index state.
- A successful update refreshes a collection's stored files and every
  configured index: lexical and graph indexes, plus the semantic index when
  enabled. The collection's content and configured indexes commit together; a
  failure leaves its prior content and indexes available unchanged.
- `update --all` attempts collections independently, reports each outcome,
  continues after failures, and exits unsuccessfully if any collection fails.
- During migration, a collection with an existing semantic index remains
  semantically enabled. A collection without one remains disabled. Migration
  does not infer any additional semantic configuration.
- `model list` reports supported embedding and re-ranker models and whether
  their assets are locally available.
- `model set [NAME] [--reranker NAME] [--download]` selects database-wide
  model settings. At least one of `NAME` or `--reranker NAME` is required.
  A setting omitted from the command remains unchanged. Model assets are
  downloaded only when `--download` is supplied.
- Changing the embedding model rebuilds every enabled semantic index, including
  those outside any collection scope. The new global setting and all affected
  semantic indexes become active together. If any rebuild fails, the prior
  setting and all prior semantic indexes remain active.
- Changing only the re-ranker model does not rebuild semantic vectors.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | A new collection without semantic opt-in | I create it and run update | Files, lexical index, and graph are built; no semantic index is built |
| EX-002 | A collection has lexical and graph indexes, but semantic indexing is disabled | I enable semantic indexing and run update | Its semantic index is built under the selected global embedding model |
| EX-003 | An enabled collection has stored semantic vectors | I disable semantic indexing and run update successfully | Its vectors and semantic index state are removed |
| EX-004 | Several collections have semantic indexes under model A | I select embedding model B | Every enabled semantic index is rebuilt under B and the global setting changes together |
| EX-005 | One collection fails during the rebuild for model B | I select embedding model B | The operation fails and model A and every prior semantic index remain active |
| EX-006 | One collection fails during `update --all` | I update all collections | Other collections are attempted, every outcome is reported, and the command exits unsuccessfully |
| EX-007 | A legacy collection has a semantic index but no semantic configuration | The database is migrated and the collection is updated | Semantic indexing remains enabled and is refreshed |
| EX-008 | A model may or may not have local assets | I list models | Supported embedding and re-ranker models are listed with local availability |
| EX-009 | The selected model is not locally available | I set it without `--download` | The operation fails before changing model settings or indexes and explains how to download it |
| EX-010 | A selected model is not locally available and downloads are allowed | I set it with `--download` | Assets are fetched, then the setting and affected indexes are updated atomically |
| EX-011 | I want to change only the global re-ranker model | I run `model set --reranker NAME` | The re-ranker setting changes without rebuilding semantic vectors or changing the embedding model |
| EX-012 | A collection has semantic indexing disabled | I run the existing `embed` command for it | The command fails with guidance to enable semantic indexing in collection configuration; the setting stays disabled |

## Acceptance Criteria

- Semantic indexing defaults to disabled for new collections and can be enabled
  or disabled through collection configuration.
- Disabling semantic indexing removes that collection's vectors and index
  state on its next successful update.
- Each successful update commits stored content and all configured indexes
  together for that collection; failures preserve the prior state.
- `update --all` continues after collection failures, reports each result, and
  returns failure if any collection failed.
- Migration preserves semantic enablement for collections that already have a
  semantic index and leaves other collections disabled.
- Model listing reports supported model names and local availability.
- Model selection affects the database-wide embedding and optional re-ranker
  settings. At least one model must be supplied; omitted settings stay
  unchanged. Assets are downloaded only when explicitly requested.
- An embedding-model change rebuilds all enabled semantic indexes. The global
  model setting and rebuilt indexes commit together or the previous
  configuration remains active.
- A re-ranker-only change does not rebuild semantic vectors.
- `model set --reranker NAME` is valid without an embedding model name and
  leaves the current embedding model unchanged.
- Until the final command-grammar slice, `embed` fails for a collection with
  semantic indexing disabled and does not change that setting.

## Scope Boundaries

### In Scope

- Per-collection semantic-index enablement and its migration behavior.
- Refreshing lexical, graph, and enabled semantic indexes in a successful
  source-aware update.
- Atomic per-collection update and atomic database-wide embedding-model change.
- Model listing, database-wide model selection, availability reporting, and
  explicit download behavior.
- Failure handling and rollback behavior for updates and model changes.
- Transitional behavior of the existing `embed` command until the final
  command-grammar slice.

### Out Of Scope

- Unified retrieval commands, search mode selection, graph query scoping, and
  richer status reporting.
- The final breaking CLI grammar and old-to-new command mapping.
- Changing semantic ranking, embedding passage selection, or supported-model
  definitions established by earlier semantic-index features.

## Dependencies

- `US-010` provides semantic-index construction, state, and model availability
  behavior.
- `US-011` provides re-ranker model support and provisioning.
- `US-015` provides dimension-aware semantic indexes.
- `US-021` provides registered sources and source-aware collection updates.
- `ADR-006`, `ADR-007`, and `ADR-010` define the existing global model,
  re-ranker, and vector-storage constraints.

## Open Questions

None. Semantic disablement, migration behavior, update atomicity, and global
model-change rollback are resolved in the approved story scope.

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
