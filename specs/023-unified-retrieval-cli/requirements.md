---
id: REQ-023
title: "Unified retrieval and final CLI requirements"
type: feature-requirements
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved the proposed REQ-023 contract in chat"
parent: US-023
depends_on:
  - US-008
  - US-009
  - US-011
  - US-013
  - US-020
  - US-021
  - US-022
requires:
  - ADR-004
  - ADR-008
  - ADR-009
  - ADR-010
  - ADR-016
  - ADR-017
blockers: []
related:
  - PRD-001
---

# Requirements

## Purpose And Actors

### Purpose

Deliver the final CLI command grammar and consistent retrieval, graph,
inspection, and output contracts without changing established ranking or
indexing behavior.

### Actors And External Systems

- Developer-curator using human output.
- Coding agent harness using machine-readable output and explicit file IDs.
- The existing local collection database and registered Markdown sources.
- The existing local model cache; network access occurs only for explicitly
  authorized model downloads during update or model selection.

## Preconditions

- US-023 and its sibling scenarios are approved.
- Existing indexing, model management, graph, and retrieval capabilities are
  available from the completed dependency features.
- Read commands require an existing database and shall not create a missing
  database. A selected collection must exist.
- Search prerequisites and supported models retain their established
  semantics; the command redesign does not change ranking or model algorithms.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Collection creation | `collection create NAME PATH... [--semantic]` | Creation outcome; sources and semantic policy persisted without indexing | Valid collection name, at least one valid source, no equivalent existing name |
| Collection configuration | `collection configure NAME [--sources PATH...] [--semantic on\|off]` | Configuration outcome without indexing | Existing collection; at least one configuration option; valid sources/policy |
| Collection listing | `collection list [--json]` | Collections with registered sources and enabled indexes | Existing database |
| Collection deletion | `collection delete NAME` | Deletion outcome | Existing database and collection; retain original source files |
| Update | `update (--collection NAME \| --all) [--download] [--skip-unreadable] [--json]` | Per-collection update outcomes | Exactly one scope; registered sources and existing update prerequisites |
| Search | `search QUERY... [--mode lexical\|hybrid] [-c NAME] [-n N] [--json] [--related] [--no-rerank]` | Ranked passages with identity/provenance or an explicit empty result | Nonempty literal query; supported mode; limit 1–100; no-rerank only in hybrid mode |
| File retrieval | `get PATH_OR_NAME -c NAME` or `get --id ID -c NAME` | Exact stored bytes | Exactly one selector; explicit ID is a positive integer within the supported ID range |
| Status | `status [-c NAME] [--json]` | Enabled indexes, readiness, stored-content freshness, model information, and last successful build times | Existing database and selected collection when supplied |
| Model catalog/selection | Existing `model list [--json]` and `model set [NAME] [--reranker NAME] [--download]` | Existing catalog and atomic model-selection outcomes | Preserve REQ-022 validation and explicit download behavior |
| Graph neighbors | `graph neighbors KEY -c NAME [--kind file\|tag\|alias] [--relation RELATION] [--depth N]` | Neighbors with their kinds, keys, titles, relations, and depths | Existing node and collection; supported kind/relation; depth 1–255 |
| Graph query | `graph query QUERY -c NAME` | Existing graph query result encoded as JSON, bound to the selected collection | Valid GraphQL; collection arguments on public fields are unsupported |

The global `--database PATH` option shall work before, between, and after
command/subcommand names and arguments. Applicable commands accept
`-c/--collection`; search accepts `-n/--limit`.

## Functional Requirements

| ID | Requirement | Priority | Traceability (Story & Scenario) |
| --- | --- | --- | --- |
| FR-001 | The CLI shall expose only the approved final command grammar. Transitional `collection add/destroy/update`, `embed`, `hybrid`, `index status`, and `context` commands shall be rejected with argument-error behavior. | Must | US-023 / The normal workflow uses the final command grammar; Obsolete commands are rejected |
| FR-002 | Collection creation shall require at least one source and preserve default-off semantic policy. Creation and configuration shall register sources/policy without indexing; configuration shall accept source and semantic changes independently or together. | Must | US-023 / Collection creation requires at least one source; Collection configuration does not index content |
| FR-003 | Human and JSON collection listing shall report canonical registered sources and enabled lexical, graph, and semantic indexes. Existing `name` and `sources` JSON fields shall remain present. | Must | US-023 / Collection listing reports sources and enabled indexes |
| FR-004 | Collection deletion shall preserve complete atomic stored-data cleanup and shall retain original source files. | Must | US-023 / Collection deletion retains original files |
| FR-005 | Top-level update shall require exactly one scope selector and preserve explicit download permission, skip-unreadable behavior, registered-source discovery, and per-collection atomicity. | Must | US-023 / The normal workflow uses the final command grammar; Update requires exactly one scope selector |
| FR-006 | JSON update reports shall contain every attempted collection's outcome, including added, modified, deleted, skipped, and malformed-frontmatter counts on success and a diagnostic on failure. Update-all shall continue after failures and return exit 1 if any collection fails. | Must | US-023 / Structured updates retain atomic per-collection outcomes |
| FR-007 | Successful structured update reports shall be one JSON document on stdout. On update failure the structured outcome report shall be one JSON document on stderr with stdout empty, preserving the operational-failure stream convention. | Must | US-023 / Structured updates retain atomic per-collection outcomes; Diagnostics and informational commands retain their stream contract |
| FR-008 | Search shall accept one or more positional query arguments, join separate arguments with a single space, and interpret the resulting string as the existing literal free-text query. Quoted and equivalent multiple-word input shall produce the same ranked passages. Empty/whitespace-only queries remain invalid. | Must | US-023 / Quoted and multiple-word queries are equivalent |
| FR-009 | Search mode shall default to lexical, accept only lexical or hybrid, and dispatch to existing lexical ranking or hybrid fusion/re-ranking respectively. Existing lexical contributions from collections without semantic indexes remain supported. | Must | US-023 / Search defaults to lexical mode; Hybrid mode preserves existing retrieval behavior |
| FR-010 | Search shall preserve related-file enrichment without changing ranked results, adding the established related output only when requested. | Must | US-023 / Search retains related-file enrichment |
| FR-011 | Search shall accept no-rerank only in hybrid mode; its use with explicit or default lexical mode shall be an argument error. | Must | US-023 / Disabling re-ranking requires hybrid mode |
| FR-012 | Actual hybrid prerequisite failures shall identify the missing or invalid prerequisite and provide a command using the replacement grammar to recover. Search shall never download model assets. Existing best-effort reranker behavior remains supported. | Must | US-023 / Hybrid prerequisite failures include a recovery command |
| FR-013 | Search limits shall default to 10, accept 1–100 inclusive, and reject values outside that range as argument errors. | Must | US-023 / Search limits retain their bounds; Search uses the default limit of ten |
| FR-014 | Each human search result shall identify its collection and explicit file ID while retaining its passage text, path, kind, score, and position. Empty results in either mode shall explicitly report no matches. | Must | US-023 / Human search results identify collection and file; Empty searches produce explicit output |
| FR-015 | Search JSON shall preserve all existing fields and meanings for its selected mode, add top-level `mode` with lexical or hybrid, and add numeric positive `file_id` to each result. Empty JSON results shall remain well formed with zero total and an empty results array. | Must | US-023 / Search JSON extends existing contracts; Empty searches produce explicit output |
| FR-016 | Get shall require a collection and exactly one path/name or explicit-ID selector. Numeric positional input shall be resolved as a path/name rather than inferred as an ID. | Must | US-023 / Numeric names remain name selectors; Retrieval rejects conflicting selectors |
| FR-017 | Get shall emit exactly the stored bytes, including empty content, existing trailing newlines, and non-UTF-8 bytes, without adding a newline or performing text conversion. | Must | US-023 / Retrieval emits exact stored content |
| FR-018 | Get shall preserve exact-path-first lookup, unique-basename fallback, scoped ID lookup, file-not-found errors, and ambiguity errors with candidate paths. | Must | US-023 / Retrieval preserves scoped not-found and ambiguity errors |
| FR-019 | Both graph commands shall require collection selection. Neighbor inspection shall default to file kind and depth 1 and shall accept explicit kind, relation, and depth selectors. It shall return data only from the selected collection. | Must | US-023 / Graph commands require collection selection; Neighbor inspection defaults to file nodes and one hop; Explicit neighbor options restrict traversal |
| FR-020 | Neighbor depth shall accept 1–255 inclusive and reject values outside that range. Node kind shall accept file, tag, or alias; relation filters shall accept existing relation kinds and reject unsupported values. | Must | US-023 / Neighbor depth retains its bounds; Explicit neighbor options restrict traversal |
| FR-021 | Graph query shall bind node and neighbor resolvers to the CLI-selected collection. Public graph fields shall not accept collection arguments; attempts to supply them shall fail query validation without returning graph data. | Must | US-023 / Graph queries are bound to the selected collection; GraphQL cannot override CLI collection selection |
| FR-022 | Status shall support all collections or one selected collection and human or JSON output. It shall report each index's enablement, readiness, freshness, model metadata where applicable, and last successful build time. An index never built shall have no successful-build timestamp. | Must | US-023 / Status reports readiness, enablement, models, and successful builds; Status can be restricted to a collection |
| FR-023 | Index freshness shall compare the index's built content with current stored collection content. A mismatch shall be reported stale while retaining the previous successful-build time. Status shall not scan sources or classify pending filesystem edits as stored-content changes. | Must | US-023 / Freshness compares indexes with stored content; Pending filesystem changes do not affect status freshness |
| FR-024 | Global database options and applicable collection/limit short options shall select the same scope regardless of command depth. HOME shall be resolved only when an operation actually needs a default path requiring it. | Must | US-023 / Global options work throughout the command tree; Explicit paths permit operations without unnecessary HOME resolution |
| FR-025 | Help and version shall succeed on stdout with exit 0 without HOME. Argument errors shall use stderr and exit 2; operational errors shall use stderr and exit 1, including JSON update failure reports. | Must | US-023 / Diagnostics and informational commands retain their stream contract |
| FR-026 | Every command shall have descriptive help and examples reflecting defaults and prerequisites. Documentation shall publish the complete old-to-new mapping and the replacement workflow. | Must | US-023 / Help and migration documentation describe the replacement surface |
| FR-027 | Existing model listing/selection, atomic updates, collection migrations, offline behavior, ranking, and provenance semantics shall be preserved except for the expressly approved CLI and output changes. | Must | US-023 / The normal workflow uses the final command grammar; Hybrid mode preserves existing retrieval behavior; Structured updates retain atomic per-collection outcomes |

### Search JSON Compatibility

Both modes preserve `query`, `scope`, `limit`, `total`, and `results`. Each
result preserves `collection`, `path`, `kind`, `text`, and `position` with
`byte_offset`, `byte_length`, `line_start`, and `line_end`.

- Lexical results retain `score` and its established BM25 meaning.
- Hybrid results retain `reranker_score`, `fused_score`, `bm25_score`,
  `cosine_similarity`, and `ordering_score`; the top-level `reranked` and
  `rerank_warning` fields remain present.
- When requested, `related` retains its existing per-result format.
- The new `mode` field identifies the selected search mode, and `file_id`
  identifies the stored file used by the explicit get selector.

## Postconditions And Invariants

- Search, get, graph inspection, model listing, and status do not alter stored
  collection content or indexes and do not perform model downloads.
- Get output may be raw bytes; machine-readable reports are complete JSON
  documents, not mixed with human diagnostic text on their report stream.
- Search provenance remains tied to stored file content and stable file IDs.
- GraphQL collection scope is imposed by the invocation and cannot be changed
  by fields, aliases, or multiple roots in the supplied query.
- Indexing failure continues to preserve each failed collection's previous
  files and indexes; global model-change failure preserves the previous global
  configuration and affected semantic indexes.
- Source registration, semantic policy changes, and pending filesystem edits
  do not themselves modify stored content or successful-build timestamps.
- No model is downloaded without an existing explicit download switch.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Obsolete command, invalid mode/kind/relation, invalid numeric limit/depth/ID, conflicting or missing selector | Reject before performing the requested mutation | Argument diagnostic on stderr; exit 2 |
| Empty or whitespace-only search query | Preserve existing query validation | Clear error; no result document |
| Missing database on a read operation | Fail without creating a file | Operational diagnostic; exit 1 |
| Unknown selected collection or graph node | Preserve scoped lookup errors | Clear operational diagnostic; exit 1 |
| Missing file or ambiguous basename | Preserve existing retrieval errors | File-not-found or candidate paths; exit 1 |
| Search has no matches | Succeed | Explicit human no-matches message or valid empty JSON |
| Required hybrid asset unavailable, semantic state stale, or model dimension inconsistent | Preserve existing failure semantics and add replacement-grammar guidance | Operational diagnostic with recovery command; exit 1 |
| Reranker unavailable in an otherwise valid hybrid query | Preserve existing best-effort warning and no-rerank behavior | Retrieval result and existing warning semantics |
| Any update-all collection fails | Continue processing other collections; preserve each failed collection's old state | Complete per-collection report on stderr; exit 1 |
| Index never built | Report unbuilt readiness | No successful-build timestamp |
| Index differs from stored content | Report stale freshness | Preserve prior successful-build metadata |
| Source unavailable or edited during status | Do not inspect sources | Report solely from stored content and index data |
| HOME missing but required paths supplied or unused | Continue without resolving HOME | No HOME-related failure |

## Quality Requirements

- Use existing architectural layers and dependencies; add no workspace member,
  crate-level dependency, or architectural layer.
- Preserve deterministic query semantics, ranking, tie-breaking, related
  enrichment, and correct stored-content positions.
- Verify subprocess bytes, output streams, and exit codes, including help
  without HOME, binary get output, and structured update failures.
- Test-first implementation shall cover every approved scenario, including
  stale legacy indexes, same-key cross-collection graphs, numeric filenames,
  and unchanged search JSON fields.
- Run `cargo xtask validate-specs`, `cargo xtask ci`,
  `cargo xtask eval --verify-only`, `cargo xtask eval`, and `git diff --check`.
- Preserve ADR-004 thresholds: Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75.
- Status freshness shall not depend on source access or filesystem timestamps.

## Dependencies And Deferred Decisions

- US-008/US-009 establish output and stored-file retrieval contracts.
- US-011/US-013 establish hybrid ranking and graph context behavior.
- US-020 establishes parser diagnostics, global options, and path resolution.
- US-021/US-022 establish source-aware indexing, semantic policy, and atomic
  updates/model management.
- Representation of raw command output, index-freshness persistence, graph
  context binding, and serialization of new status/update reports belong in
  DES-023 and any associated approved ADRs.
- No blocking product or behavioral questions remain.

## Traceability

- Source story: US-023 in `user-story.md`.
- Executable behavior: sibling `scenarios.feature`; every scenario is covered
  by one or more FR rows above.
- Parent product: PRD-001, primarily EPIC-006 with collection, indexing, search,
  semantic, and graph-context capabilities.
- The expressly approved breaking grammar and output changes replace earlier
  CLI contracts; existing ranking and storage invariants remain applicable.
