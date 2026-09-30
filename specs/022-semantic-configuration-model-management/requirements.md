---
id: REQ-022
title: "Semantic configuration and model management requirements"
type: feature-requirements
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the REQ-022 contract in chat on 2026-09-30"
parent: US-022
depends_on:
  - US-010
  - US-011
  - US-015
  - US-021
requires: []
blockers: []
related:
  - DES-010
  - DES-011
  - DES-015
  - ADR-006
  - ADR-007
  - ADR-010
  - ADR-012
  - DB-001
---

# Requirements

## Purpose And Actors

### Purpose

Let users enable semantic indexing for selected collections, maintain each
collection's chosen indexes through update, and manage the database-wide
embedding and re-ranker models without exposing collections to partially
rebuilt index state.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- The local collection database containing stored files and indexes.
- The local embedding and re-ranker model cache.
- The network, used only when the user explicitly requests model downloads.

## Preconditions

- Collection operations use the selected existing database.
- Collection configuration targets an existing collection.
- Source-aware updates use registered sources as defined by US-021.
- Semantic indexing uses the supported embedding and re-ranker model sets
  established by US-010 and US-011.
- Model downloads are opt-in and require an explicit `--download` switch.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Create a collection | `collection create NAME PATH...`, optional `--semantic` | Collection and registered sources are created; no files are indexed | Name and source validation follow US-001 and US-021; semantic indexing defaults off |
| Configure a collection | `collection configure NAME`, optional `--sources PATH...`, optional `--semantic on\|off` | Updated source and semantic configuration | Collection must exist; configuration alone does not ingest or index content |
| Update one collection | `update --collection NAME` and existing update switches | Per-collection update outcome | Enabled semantic indexing uses the selected global model |
| Update all collections | `update --all` and existing update switches | Outcome for every collection and command failure if any collection fails | Each collection is attempted independently |
| List models | `model list` | Supported embedding and re-ranker models with local availability | Listing performs no downloads or database changes |
| Set models | `model set [NAME] [--reranker NAME] [--download]` | Model-change outcome and affected collection outcomes | At least one model input is required; names must be supported; omitted settings remain unchanged; downloads require `--download` |
| Embed using the legacy command | Existing `embed` arguments | Semantic index outcome for targeted collections | A targeted collection must have semantic indexing enabled through collection configuration |

## Functional Requirements

| ID | Requirement | Priority | Traceability (Story & Scenario) |
| --- | --- | --- | --- |
| FR-001 | A newly created collection shall have semantic indexing disabled unless `--semantic` explicitly enables it. Creating or configuring a collection shall not ingest files or build indexes. | Must | US-022 / Scenario: A new collection has semantic indexing disabled by default |
| FR-002 | Collection configuration shall allow semantic indexing to be enabled or disabled. Enabling it shall cause the next successful update to build its semantic index. | Must | US-022 / Scenarios: Enabling semantic indexing builds it on update; Disabling semantic indexing removes its index on successful update |
| FR-003 | When semantic indexing is disabled, the next successful update shall remove that collection's vectors and semantic index state. | Must | US-022 / Scenario: Disabling semantic indexing removes its index on successful update |
| FR-004 | A successful collection update shall commit stored file changes and all configured indexes (lexical, graph, and semantic when enabled) together. On any indexing failure, the collection's prior content and indexes shall remain unchanged. | Must | US-022 / Scenarios: Enabling semantic indexing builds it on update; Disabling semantic indexing removes its index on successful update; A failed update preserves a disabled collection's previous index state |
| FR-005 | `update --all` shall attempt every collection independently, report every collection's outcome, continue after failures, and return a failure status if any collection fails. | Must | US-022 / Scenario: Update all continues after a collection failure |
| FR-006 | Migration shall preserve semantic enablement for each legacy collection with an existing semantic index. A collection without an existing semantic index shall remain disabled. | Must | US-022 / Scenario: Migration preserves existing semantic enablement |
| FR-007 | `model list` shall report supported embedding and re-ranker models and whether each model's assets are locally available. Listing shall not download assets or change database settings. | Must | US-022 / Scenario: Model list reports support and local availability |
| FR-008 | `model set [NAME] [--reranker NAME] [--download]` shall require an embedding model name, a re-ranker name, or both. Any omitted model setting shall remain unchanged. | Must | US-022 / Scenarios: Changing the embedding model rebuilds every enabled semantic index; Selecting only a re-ranker does not rebuild semantic vectors; Setting neither model is rejected |
| FR-009 | Model names shall be validated against the supported model sets. Model assets shall only be downloaded when `--download` is supplied to model selection. | Must | US-022 / Scenarios: An unavailable model cannot be selected without download permission; Model selection downloads only when explicitly allowed |
| FR-010 | A model validation, local-availability, or download failure shall occur before database model settings or collection indexes are changed. The output shall identify the unavailable or invalid model and explain the explicit download recovery path when applicable. | Must | US-022 / Scenarios: An unavailable model cannot be selected without download permission; Model selection downloads only when explicitly allowed; Setting neither model is rejected |
| FR-011 | Changing the global embedding model shall rebuild every semantic-enabled collection, regardless of any collection scope supplied to the command. The new model setting and all affected semantic indexes shall commit together; if any rebuild fails, the previous model setting and every prior semantic index shall remain active. | Must | US-022 / Scenarios: Changing the embedding model rebuilds every enabled semantic index; A failed model change preserves the prior global model and indexes |
| FR-012 | Changing only the global re-ranker model shall leave the embedding model and all semantic vectors unchanged. | Must | US-022 / Scenario: Selecting only a re-ranker does not rebuild semantic vectors |
| FR-013 | Until the final command-grammar slice, the existing `embed` command shall fail for a collection whose semantic indexing is disabled, provide guidance to enable it through collection configuration, and leave the setting unchanged. | Must | US-022 / Scenario: The legacy embed command respects collection semantic configuration |

## Postconditions And Invariants

- A collection's semantic-index configuration is persisted and governs future
  updates.
- A successful update leaves the collection's stored content and every
  configured index describing the same committed file set.
- A failed update leaves the collection's previous stored content and indexes
  available unchanged.
- A collection whose semantic indexing is disabled has no stored semantic
  vectors or semantic-index state after its next successful update.
- Legacy semantic indexes remain enabled across migration; migration does not
  guess enablement for collections without an index.
- At most one global embedding model and one global re-ranker model are
  effective in a database.
- After a successful embedding-model change, every semantic-enabled collection
  has an index built with that model and its expected dimension.
- A failed embedding-model change leaves the previous global model and all
  prior semantic indexes active.
- A re-ranker-only change does not modify the embedding model or vector indexes.
- Model listing and model configuration do not download assets unless the user
  explicitly supplies `--download`.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Semantic indexing omitted when creating a collection | Persist semantic indexing as disabled | Collection creation succeeds without building indexes |
| Semantic indexing disabled for an indexed collection | Remove vectors and semantic state on its next successful update | Successful update reports its result |
| Any configured index fails during update | Roll back that collection's files and indexes | Collection update is reported as failed |
| A collection fails during update-all | Continue with remaining collections | All outcomes are reported; command returns failure |
| Legacy collection has semantic index state | Preserve semantic enablement and refresh it on update | Existing semantic behavior continues |
| Legacy collection has no semantic index state | Keep semantic indexing disabled | No semantic index is built until enabled |
| A model input is unsupported | Fail before database mutation or indexing | Error identifies the unsupported model |
| A model is not available locally and download was not allowed | Fail before database mutation or indexing | Error identifies the model and how to request its download |
| A model download fails | Preserve prior database settings and indexes | Download failure is reported |
| Any collection rebuild fails during global embedding-model change | Roll back the model setting and all affected indexes | Model change fails; previous configuration remains active |
| Only a re-ranker model is supplied | Update only the global re-ranker setting | Embedding setting and vectors remain unchanged |
| `model set` receives neither embedding nor re-ranker model | Reject the command as invalid input | Argument error explains that at least one model is required |
| The existing `embed` command targets a semantically disabled collection | Fail without changing the collection setting | Error guides the user to configure semantic indexing first |

## Quality Requirements

- Model inference and index maintenance work locally by default; network use is
  limited to an explicit download request.
- Index updates remain explicit; no file watcher or background refresh is
  introduced.
- Model changes and per-collection updates preserve the last committed,
  searchable state after a failure.
- No new dependency or external service is required for this feature.

## Traceability

- Source story: `US-022` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
- Existing model and vector decisions: `ADR-006`, `ADR-007`, `ADR-010`,
  and `ADR-012`
