---
id: DES-023
title: "Unified retrieval and final CLI design"
type: feature-design
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin approved DES-023 and ADR-018/019 in chat"
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
  - ADR-018
  - ADR-019
blockers: []
related:
  - REQ-023
  - PRD-001
  - DB-001
---

# Design

## Context And Constraints

REQ-023 finalizes the approved breaking CLI redesign after source-aware and
semantic-aware atomic updates. Preserve segmentation, ranking, fusion,
reranking, related enrichment, stable file identity, and transaction behavior.
Use existing crates, dependencies, and architectural layers. There is no
database migration in this slice; DB-001 remains at schema version 9.

Follow the Constitution's typed port/error boundaries, concrete composition,
function/file size limits, and RED -> GREEN -> REFACTOR -> TRACE cycle. Split
touched oversized dispatch/rendering code by existing responsibilities rather
than adding an architectural layer. This design does not authorize dependencies,
unsafe code, lint relaxation, or unrelated refactoring.

## Proposed Design

Replace the parser surface with collection create/configure/list/delete,
top-level update, unified search, explicit get selectors, status, model list/set,
and graph neighbors/query. Clap enforces required/exclusive selectors, supported
values, limit/depth ranges, and global database placement before dispatch.
Validate mode-dependent no-rerank usage as a clap argument error before opening
the database. Join search query words once and feed the result to the existing
literal-query use cases.

Introduce an app-level command output type with report text and exact stored
bytes as distinct variants. Both invocation entry points return this type.
The executable writes a report with its normal final newline, but writes
content with write_all and no conversion or added bytes, even when empty or
non-UTF-8. Clap retains control of help/version and argument diagnostics.

Add explicit application file selectors for path/name and validated FileId.
Path/name lookup retains exact-path-first and unique-basename fallback; it
never infers an ID from numeric text. Extend lexical search results with their
stored FileId from the existing SQL join. Hybrid results expose the existing
passage file identity. Render additive mode/file_id JSON fields and human
identity headers without changing order or scores.

Read index status from one SQLite snapshot using ADR-018. Bind graph queries
through invocation context using ADR-019. Model management and all mutation
transactions retain their existing application and adapter ownership.

## Components And Responsibilities

| Component | Responsibility | Depends on |
| --- | --- | --- |
| App CLI parser | Final grammar, examples, global options, typed modes/kinds/selectors, argument validation | Existing clap dependency |
| App command output and binary | Report/content distinction, exact byte writes, exit/stream conventions | App errors and existing I/O |
| App command composition | Resolve only needed default paths; wire existing use cases and adapters | Application ports, infrastructure, adapters |
| App result/report renderers | Preserve search fields; add identity/mode; serialize collection, update, and status reports | Application result types, serde_json |
| GetFile | Explicit scoped selector and existing path/basename/ID lookup behavior | FileRetrievalStore |
| Search use cases and stores | Existing ranking and enrichment with stable file IDs in results | Existing lexical/hybrid ports and SQLite queries |
| ReadIndexStatus and status port | Scoped status report, enabled indexes, readiness, model/build metadata | Narrow application-owned read port |
| SQLite status adapter | Consistent snapshot, legacy-aware reads, stored-content comparisons | Existing tables and domain segmentation/fingerprints |
| Graph query root | Context-bound collection, concrete synchronized store, scoped resolvers | Existing async-graphql and GraphStore |
| Existing update/model coordinators | Stage inference and atomically commit configured index/model changes | Existing atomic ports from US-021/022 |

## Interfaces And Contracts

| Interface | Inputs | Outputs | Errors |
| --- | --- | --- | --- |
| Command runner | Arguments plus explicitly supplied home or process path environment | Report(String) or Content(Vec<u8>) | Typed AppError, including clap diagnostics and complete failed update report |
| GetFile | CollectionName and explicit path/name or FileId selector | RetrievedFile containing original bytes | Existing missing/ambiguous/storage errors |
| Unified search dispatch | Joined literal query, mode, optional collection, limit, related/rerank policy | Existing selected-mode result set enriched with FileId | Existing typed retrieval/prerequisite errors with final-grammar recovery guidance |
| Status read port | Optional CollectionName scope | Global model settings and collection/index status values | Existing typed status/storage errors; selected collection not found |
| Bound graph schema | Concrete GraphStore, CollectionName, GraphQL document | Existing node/neighbor JSON shapes | Query validation, unknown node, storage/lock errors |
| Update report builder | Ordered per-collection success/failure outcomes | One human or JSON report | Failed outcomes retained in operational error payload |

Status distinguishes disabled, never built, built/current, and built/stale
indexes. Each index reports enablement, readiness, freshness when built,
successful-build timestamp or null, and applicable counts/model metadata.
Global selected embedding and reranker names are reported separately from
per-index embedding model and dimension. A stale built index retains its
timestamp. A disabled semantic policy can temporarily retain prior index
metadata until update removes vectors/state. Do not treat policy changes as
successful builds or pending filesystem changes as stored-content changes.

For legacy schemas, absence of index tables/state means unbuilt; absent source
registration means no registered sources; semantic enablement before policy
schema version 9 is inferred only from existing semantic state, matching the
approved migration policy. Read commands do not apply that migration. Decode
only columns available at the recorded schema version.

Search JSON keeps its existing selected-mode fields and adds top-level mode
and each result's positive numeric file_id. Related output remains conditional.
Human empty searches explicitly report no matches. Collection list keeps name
and sources and adds enabled indexes. Update JSON contains every attempted
collection, success counts (added, modified, deleted, skipped, malformed
frontmatter), and failure diagnostics. Serialization completes before output,
so stdout/stderr never contains a partial or mixed document.

## Data And State Flow

1. Parse and validate arguments before resolving HOME or opening a database.
   Resolve database defaults only when needed. Resolve model-cache defaults
   only when an operation needs model assets: lexical-only updates must work
   with an explicit database and no HOME. Preserve explicit cache overrides.
2. Read commands open an existing database without migrations or initialization.
   Read collection configuration/status with legacy-aware queries. Missing
   databases fail without creating a file; read commands do not mutate files,
   index state, or settings.
3. Search dispatches to the unchanged mode-specific use case, enriches only
   when requested, and renders one complete report. Hybrid prerequisites retain
   their established semantics, including lexical-only contributions and
   best-effort reranker warnings. Guidance uses update/model/collection configure
   commands and never performs search downloads.
4. Get resolves the explicit selector within the chosen collection, then passes
   the RetrievedFile bytes directly to the binary output writer.
5. Status opens a read snapshot, reads settings/policies/build records and stored
   files, compares lexical passages/mappings and graph/semantic fingerprints,
   checks model/dimension compatibility, and emits a complete report. It never
   opens registered sources or model assets. Drop the read snapshot on success
   or failure without changing persistent state.
6. Graph commands validate collection existence. Neighbors use explicit kind,
   relation, and depth; query injects CollectionName into schema context and
   rejects public collection arguments before resolving fields.
7. Update retains source discovery, staging, and the existing per-collection
   transaction. Accumulate every update-all outcome in deterministic collection
   order. Success goes to stdout; any failure sends the complete report to stderr
   and returns exit 1. Individual failed transactions preserve prior state while
   other collections continue. Global model changes retain their atomic settings
   and semantic-index transaction.

## Security, Performance, And Operations

- Collection context is imposed before graph resolution; aliases, variables,
  fragments, and multiple roots cannot override it. No external server is exposed.
- Read-only inspection avoids migrations and filesystem source access. No
  model download occurs without an existing explicit download option.
- Lexical status costs stored-content segmentation and passage comparison;
  graph/semantic comparisons reuse existing fingerprint semantics. Search adds
  identity from existing joins rather than doing one extra lookup per result.
- No model inference runs in a SQLite write transaction. Existing update and
  model rollback behavior remains intact.
- Release is deliberately breaking. README documents every obsolete command's
  replacement, explicit file IDs, graph scope, mode/defaults, and status's
  stored-content freshness boundary.

## Alternatives Considered

| Alternative | Why not chosen |
| --- | --- |
| Keep old commands as aliases | Approved final surface explicitly rejects transitional commands |
| Keep String-only output or convert invalid bytes lossily | Cannot satisfy exact retrieval, empty content, or binary output |
| Infer numeric positional arguments as IDs | Conflicts with explicit selectors and numeric filenames |
| Persist or backfill lexical fingerprints | Requires migration and risks certifying stale legacy indexes; ADR-018 uses direct comparison |
| Compare only index counts or timestamps | Cannot detect changed content with unchanged passage count |
| Keep or rewrite GraphQL collection arguments | Duplicates scope or is fragile; ADR-019 binds typed context |
| Reimplement retrieval under one ranking algorithm | Would change established lexical/hybrid behavior and evaluation contracts |

## Risks And Open Decisions

- Legacy columns and tables differ; tests must exercise each relevant schema
  boundary without making read operations migrate.
- Output-type and command-grammar changes touch existing tests. Migrate their
  invocations and expressly superseded expectations while preserving assertions
  for unchanged ranking, JSON fields, atomicity, and lookup errors.
- Lexical status can be expensive for large stored corpora. Keep comparison
  deterministic and avoid unnecessary duplicated materialization.
- Recovery messages must accurately distinguish asset, freshness, dimension,
  and unbuilt-index failures without triggering downloads.
- No blocking decisions remain. ADR-018/019 and this design are approved.

## Verification Approach

Write and observe failing tests before each production change, with REQ-023
and named-scenario links. Cover parser bounds/exclusivity, obsolete commands,
global options at every depth, subprocess stdout/stderr/exit codes without HOME,
exact get bytes (empty, trailing newline, non-UTF-8), explicit/scoped IDs and
numeric names, unchanged ranking and JSON fields, related enrichment, hybrid
prerequisites/recovery, graph scope overrides/aliases/multiple roots, and
read-only stored-content status for current/stale/unbuilt/disabled/legacy states.

Integration checks preserve atomic update-all and global-model behavior;
structured failures must retain all outcomes and leave failed state untouched.
Existing retrieval regression tests and ADR-004 evaluation thresholds remain
mandatory. Map all sibling Gherkin scenarios to tests and record observed RED,
GREEN, refactor/trace, and quality evidence through verify-feature.

Run cargo xtask validate-specs, cargo xtask ci, cargo xtask eval --verify-only,
cargo xtask eval, and git diff --check. Never claim completion from planned gates.
