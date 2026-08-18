---
id: DES-010
title: "Build the semantic index with the embed command design"
type: feature-design
status: implemented
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: US-010
related:
  - REQ-010
  - US-006
  - DES-006
  - ADR-001
  - ADR-003
  - ADR-004
  - ADR-005
  - ADR-006
  - ADR-007
  - DB-001
  - TABLE-004
  - TABLE-005
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Design

## Context And Constraints

This feature adds the dedicated `mdsearch embed` command that builds and
maintains a per-passage semantic (vector) index over the passages the lexical
index already covers. The implementation must preserve the approved behavior in
`REQ-010` while respecting the PRD and constitution constraints:

- The application is a local-first Rust single binary; embedding works offline
  by default and the network is used only when `--download` is passed.
- All Rust implementation must comply with `specs/CONSTITUTION.md`.
- The default database is `~/.mdsearch/collections.db`, with `--database PATH`
  as an override.
- `embed` requires a built lexical index per collection (REQ-011, REQ-012);
  embedding reuses the lexical index's passage set.
- `update` fully rebuilds the lexical index on every run (ADR-005), destroying
  and recreating FTS5 passage rowids even when file content is unchanged.
  Semantic vectors therefore key to a stable logical passage identity
  `(file_id, kind, position)` instead of the physical passage rowid, so a
  no-op `update` does not invalidate the semantic index.
- Vectors are stored in the vendored `sqlite-vector` extension in the same
  SQLite file (ADR-001, ADR-003).
- Embeddings are generated locally by `fastembed` with the approved default
  model `all-MiniLM-L6-v2` (384 dimensions, cosine metric).
- Model availability is validated before any collection work; a missing local
  model fails clearly unless `--download` is passed (REQ-009, REQ-010).
- A single global embedding model is recorded in the database; a `--model`
  switch rebuilds every collection with an existing semantic index (REQ-006,
  REQ-007).
- The hybrid search slice (US-011, ADR-007) provisions the local cross-encoder
  re-ranker through `embed --reranker NAME`; `embed` validates the re-ranker
  name, checks or fetches its assets, and records a single global re-ranker
  model in the existing `settings` table (REQ-019 through REQ-021). The
  re-ranker stores no vectors and modifies no collection's semantic index.

## Proposed Design

Add an embedding generator port, a re-ranker port, a semantic index store port,
a use case, a CLI command, a `fastembed` adapter crate, and a schema-v5
migration.

- The domain gains `EmbeddingModel` (a validated non-empty model name),
  `Embedding` (a `Vec<f32>` vector), `SemanticPassage` (file, kind, position,
  text), a pure `file_set_fingerprint` function that hashes a collection's
  stored file set so staleness can be detected without re-reading disk, and
  `RerankerModel` (a validated non-empty re-ranker model name).
- The `EmbeddingGenerator` port (application) exposes model availability checks
  and text embedding; the `Reranker` port (application) exposes re-ranker model
  availability checks and (query, document) re-scoring; the `SemanticIndexStore`
  port (application) exposes the global embedding and re-ranker models,
  per-collection semantic state, passage reads, and an atomic per-collection
  rebuild.
- The `EmbedCollections` use case validates the embedding and re-ranker models,
  resolves the target collections, applies staleness and model-change logic,
  and produces a per-collection report.
- The `FastembedGenerator` adapter (new crate `crates/adapters/embed-fastembed`)
  implements the generator with `fastembed`, gating downloads behind
  `--download` and mapping model names to fastembed's supported models. The
  same crate's `FastembedReranker` implements the re-ranker with fastembed's
  `TextRerank` (ADR-007), sharing the same cache layout and download gating.
- The `SqliteSemanticIndexStore` adapter implements the store with a schema-v5
  migration: a `settings` table for the global embedding and re-ranker models,
  `semantic_index_state` per collection, and an `embeddings` vector virtual
  table with metadata columns keyed to the logical passage identity.
- The CLI adds `mdsearch embed [--collection NAME] [--model NAME]
  [--reranker NAME] [--download] [--database PATH]` and renders the
  per-collection summary.

## Components And Responsibilities

| Component | Responsibility | Depends on |
| --- | --- | --- |
| `EmbeddingModel`, `Embedding`, `SemanticPassage`, `RerankerModel` (domain) | Value types for models, vectors, passages to embed, and re-ranker models | `domain` types |
| `file_set_fingerprint` (domain) | Hash a collection's stored file set for staleness | `ContentHash`, `std` |
| `EmbeddingGenerator` port | Check model availability (with optional download) and embed texts | `domain` types |
| `Reranker` port | Check re-ranker model availability (with optional download) and re-score (query, document) pairs | `domain` types |
| `SemanticIndexStore` port | Read/write global embedding and re-ranker models, per-collection state, passage rows; atomic rebuild | `domain` types |
| `EmbedCollections` use case | Validate models, resolve targets, apply staleness/model-change logic, report | `EmbeddingGenerator`, `Reranker`, `SemanticIndexStore`, `Clock` |
| `FastembedGenerator` (embed-fastembed) | Implement the generator with `fastembed`, cache-check, download gating | `fastembed`, `EmbeddingGenerator` |
| `FastembedReranker` (embed-fastembed) | Implement the re-ranker with `fastembed`'s `TextRerank`, cache-check, download gating | `fastembed`, `Reranker` |
| `SqliteSemanticIndexStore` (store-sqlite) | Implement the store over schema-v5 tables | `rusqlite`, `sqlite-vector` |
| CLI command handler | Accept `embed`, validate inputs, render summary, signal partial failure | CLI parser and use case |

## Interfaces And Contracts

| Interface | Inputs | Outputs | Errors |
| --- | --- | --- | --- |
| `EmbeddingGenerator::ensure_available` | `&EmbeddingModel`, `download: bool` | `()` | `UnsupportedModel`, `ModelNotCached`, `DownloadFailed`, `Storage` |
| `EmbeddingGenerator::embed` | `&EmbeddingModel`, `&[&str]` texts | `Vec<Embedding>` | `EmbeddingError` |
| `SemanticIndexStore::targets` | — | `Vec<EmbedTarget>` (collection, has files, lexical built) | `SemanticIndexStoreError` |
| `SemanticIndexStore::resolve` | `&CollectionName` | `EmbedTarget` | `CollectionNotFound`, `SemanticIndexStoreError` |
| `SemanticIndexStore::global_model` | — | `Option<EmbeddingModel>` | `SemanticIndexStoreError` |
| `SemanticIndexStore::set_global_model` | `&EmbeddingModel` | `()` | `SemanticIndexStoreError` |
| `SemanticIndexStore::reranker_model` | — | `Option<RerankerModel>` | `SemanticIndexStoreError` |
| `SemanticIndexStore::set_reranker_model` | `&RerankerModel` | `()` | `SemanticIndexStoreError` |
| `SemanticIndexStore::status` | `&CollectionName` | `Option<SemanticIndexStatus>` (fingerprint, model, passage count, embedded at) | `SemanticIndexStoreError` |
| `SemanticIndexStore::embedded_collections` | — | `Vec<CollectionName>` | `SemanticIndexStoreError` |
| `SemanticIndexStore::passages` | `&CollectionName` | `Vec<SemanticPassage>` | `SemanticIndexStoreError` |
| `SemanticIndexStore::file_set_fingerprint` | `&CollectionName` | `ContentHash` | `SemanticIndexStoreError` |
| `SemanticIndexStore::rebuild` | `&CollectionName`, `&EmbeddingModel`, `Timestamp`, `&[(SemanticPassage, Embedding)]` | `usize` (passage count) | `SemanticIndexStoreError` |
| `EmbedCollections::execute` | `EmbedScope` (all / one collection), `Option<&EmbeddingModel>`, `Option<&RerankerModel>`, `download: bool` | `EmbedReport` (per-collection outcomes) | `EmbedError` (`UnsupportedModel`, `ModelNotCached`, `DownloadFailed`, `CollectionNotFound`, `IndexNotBuilt`, store/generator/clock errors) |
| `Reranker::ensure_available` | `&RerankerModel`, `download: bool` | `()` | `UnsupportedModel`, `ModelNotCached`, `DownloadFailed`, `Storage` |
| `Reranker::rerank` | `&RerankerModel`, `&str` query, `&[&str]` documents | `Vec<f64>` scores | `RerankError` |
| CLI `mdsearch embed` | `--collection NAME?`, `--model NAME?`, `--reranker NAME?`, `--download`, `--database PATH?` | Per-collection summary lines plus model used; partial-failure output and non-zero exit | "model not supported", "model not available; pass --download", "download failed", "re-ranker model not supported", "re-ranker model not available; pass --download", "collection not found", "index is not built", "database does not exist" |

`EmbedReport` carries one `EmbedOutcome` per processed collection: `Embedded`
with the passage count, `AlreadyCurrent`, `Skipped` with a reason (no files,
lexical index not built), or `Failed` with a message. `EmbedScope` selects every
collection or one named collection.

## Data And State Flow

```mermaid
flowchart TD
    INPUT["CLI: embed --collection? --model? --reranker? --download? --database?"]
    RESOLVE["Resolve effective model: --model or recorded global model or default"]
    RERANKER{"--reranker given?"}
    RAVAIL["ensure_available(reranker, download): cache check / download"]
    RSET["Record global reranker_model setting"]
    AVAIL["ensure_available(model, download): cache check / download"]
    SCOPE{"EmbedScope?"}
    TARGETS["Resolve target collections + lexical built state"]
    SWITCH{"--model differs from recorded global model?"}
    REBUILD_ALL["Mark every embedded collection for rebuild under the new model"]
    LOOP["For each target collection"]
    FILES["Compute file_set_fingerprint from stored files"]
    STALE{"Fingerprint or model differs from recorded state?"}
    SKIP["Report already current"]
    READ["Read passage rows"]
    EMBED["Generate embeddings for all passages"]
    ATOMIC["Atomic rebuild: delete old vectors, insert new, update state"]
    FAIL["Report collection failure; continue"]
    REPORT["Render per-collection summary; partial failure => non-zero exit"]

    INPUT --> RESOLVE
    RESOLVE --> RERANKER
    RERANKER -->|yes| RAVAIL --> RSET
    RERANKER -->|no| AVAIL
    RSET --> AVAIL
    AVAIL --> SCOPE
    SCOPE -->|All| TARGETS
    SCOPE -->|Collection| TARGETS
    TARGETS --> SWITCH
    SWITCH -->|yes| REBUILD_ALL --> LOOP
    SWITCH -->|no| LOOP
    LOOP --> FILES --> STALE
    STALE -->|unchanged| SKIP --> LOOP
    STALE -->|changed| READ --> EMBED --> ATOMIC --> LOOP
    ATOMIC -->|failure| FAIL --> LOOP
    LOOP -->|done| REPORT
```

Preconditions that abort the whole command: an unsupported model, a model or
re-ranker that is not cached locally without `--download`, and a failed
download. The re-ranker is validated and recorded before any collection work;
it never stores vectors. Per-collection failures during the rebuild are
reported and processing continues. Each collection's rebuild is atomic in one
transaction.

## Security, Performance, And Operations

- Security: no network access unless `--download` is explicitly passed; model
  and re-ranker assets are read from a local cache; passages are bound as
  parameters and never concatenated into SQL.
- Performance: embedding is CPU-bound inference over the collection's passage
  set, done once per rebuild; a full rebuild is O(passages), consistent with the
  PRD's unbounded indexing-time constraint. The vector table is searched only in
  the next EPIC-004 slice. Re-ranker provisioning is a one-time cache check or
  fetch that records a setting.
- Operations: schema v5 is applied by an idempotent migration when the store is
  opened for embedding; the global models and per-collection state record what
  was built and with which model, enabling the next slice's staleness reporting.
- Compatibility: `collection`, `index`, `add`, `update`, `search`, and `get`
  behavior is unchanged; existing databases migrate forward in place.

## Alternatives Considered

| Alternative | Why not chosen |
| --- | --- |
| Key vectors to the physical FTS5 passage rowid | `update` rebuilds the lexical index every run (ADR-005), so rowids change even when content is unchanged; vectors would silently dangle. The logical `(file_id, kind, position)` key is stable across no-op updates |
| Embed per document instead of per passage | Rejected in the story: semantic and lexical search must return identical passages for fusion |
| Rebuild the semantic index inside `collection update` | Rejected in the story: `embed` is a separate explicit command, keeping `update` lexical-only |
| Store the global model per collection | Rejected in the story: a single global model guarantees database-wide vector comparability; a `--model` switch rebuilds all embedded collections |
| Always rebuild, never skip | The story requires reporting already current and skipping unchanged collections |
| Re-read files from disk during `embed` | Rejected in the story: staleness compares the stored file set (content hashes), not the on-disk state |
| Provision the re-ranker through a dedicated command or flag on `hybrid` | Rejected in ADR-007: `hybrid` is read-only, so `embed --reranker` owns model-asset provisioning and records a global setting |

## Risks And Open Decisions

- The vendored `sqlite-vector` virtual table must support predicate deletes
  (`DELETE FROM embeddings WHERE collection_id = ?`); the adapter tests must
  exercise rebuild-after-rebuild before this slice is complete.
- `fastembed` model asset layout in the cache must be inspected at implementation
  time so the availability check matches fastembed's actual cache directory
  structure; the exact cache layout is adapter-internal.
- A `--model` switch rebuilds every collection with an existing semantic index
  regardless of `--collection` scope (REQ-007); the report must reflect all
  collections rebuilt, not only the targeted one.
- Partial per-collection failure must surface a non-zero exit code while still
  printing the summary; the CLI maps a report containing `Failed` outcomes to a
  distinct exit path.
- The default model `all-MiniLM-L6-v2` is the approved starting point; model
  choice is revisited through the ADR-004 evaluation framework.
- The default re-ranker `bge-reranker-base` is provisioned through
  `embed --reranker` and recorded as a global setting (ADR-007); re-ranker
  choice is revisited through the ADR-004 evaluation framework.
- `embed --reranker` reuses the embedding model's availability and download
  gating; the re-ranker adapter must match fastembed's `TextRerank` cache layout
  so the availability check agrees with a real download.

## Verification Approach

- Domain: `EmbeddingModel` validation, `Embedding` construction, `SemanticPassage`
  accessors, and `file_set_fingerprint` determinism and change-detection
  (including a total-order sort keyed on `(path, content hash)` so the
  fingerprint is deterministic even for duplicate paths).
- Application: `EmbedCollections` with in-memory fakes for every scope, model
  availability, staleness, model-switch rebuild-all, skip and fail paths, and
  report shaping.
- Store: integration tests for global model get/set, re-ranker model get/set,
  status read/write, passage reads, fingerprint computation, atomic rebuild
  (success and rollback), rebuild-after-rebuild, wrong-dimension rejection, and
  predicate delete against the vector table. The vector table metadata columns
  must be declared with double quotes (the vendored parser strips double quotes
  but not single quotes).
- Generator adapter: model availability, download gating, unsupported-model
  mapping, the approved 384-dimensional default model, and the hf-hub cache
  layout availability check. Re-ranker adapter: availability, download gating,
  unsupported-model mapping, and the hf-hub cache layout availability check.
  Real inference requires the model assets and is exercised manually/offline
  rather than in CI (no network in tests).
- CLI: acceptance tests mapped from `scenarios.feature` for the offline-reachable
  paths (missing database, unsupported model, uncached model suggesting
  `--download`), plus unit tests for the per-collection summary rendering and
  partial-failure exit.
- Run every offline-reachable scenario in `scenarios.feature` as an executable
  acceptance test; the remaining scenarios (which require model assets) are
  covered at the application layer with fakes.
- Run the Rust constitution gates from `specs/CONSTITUTION.md` before marking the
  implementation complete.
