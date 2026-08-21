# mdsearch v0.2.0 — Engineering Observations / TODO Catalogue

## Purpose

This document records the engineering observations, inconsistencies, and
technical-debt items surfaced by the full-source review of the Rust codebase.
It is the working basis for scoping the next release (`v0.2.0`) of `mdsearch`.

Each entry (`OBS-NNN`) is described in enough detail to be refined into an
epic (or a feature slice of an existing epic). It is intentionally written
independent of the open PRD question — whether `v0.2.0` should be delivered as
an **update to `PRD-001`** or as a **new PRD** — because the observations
themselves do not presuppose the answer. The PRD decision should be made with
the decision framework in `specs/prd_lifecycle_and_evolution_plan.md` once the
epic scope is agreed.

## Verified Baseline

The observations below were made against the repository at commit `0681066`
with:

- `cargo check --workspace --all-targets` — passes.
- `cargo clippy --workspace --all-targets` — clean (workspace lints, many at
  `deny`).
- `cargo test --workspace --all-targets` — **592 tests pass, 0 failures**.

The code is therefore in a healthy, fully green state; every entry below is a
quality, consistency, scalability, or correctness concern rather than a
build-breaking defect.

## How To Use This Document

1. Read each `OBS-NNN` entry; confirm the observed behavior against the cited
   locations.
2. For entries that are **product-visible behavior**, decide with the owner
   whether the intended behavior is the current one or the proposed one; do not
   invent missing requirements (SDD rule).
3. For **technical-debt** entries, decide whether they are in-scope for `v0.2.0`
   or parked.
4. When an entry becomes a committed epic, move its content into the chosen PRD
   (as an epic row and/or decision-log entry) and delete or mark it here as
   `PROMOTED`.

Legend for the **Kind** column:

- **Behavior inconsistency** — two surfaces of the product behave differently
  for the same conceptual input.
- **Architecture constraint** — a hard-coded constant or schema choice narrows
  the product's advertised capability.
- **Robustness / correctness** — fragile detection or silent fallback that could
  misreport or mask failures.
- **Scalability / performance** — algorithmic behavior that grows poorly with
  PRD-scale inputs (100–5,000 docs/collection).
- **Maintainability / DRY** — duplicated or scattered implementation.

---

## OBS-001 — Lexical and hybrid search treat the same query string differently

- **Kind:** Behavior inconsistency
- **Priority:** High (user-visible, surprising)
- **Status:** `PROMOTED` — committed as EPIC-008 in `PRD-001` (US-014,
  `specs/014-unify-query-semantics`); resolved decision recorded as DEC-013
- **Locations:**
  - `crates/application/src/hybrid_search.rs:199` — hybrid search pre-processes
    the query: `let fts5_query = free_text_to_fts5(query)…`
  - `crates/application/src/lexical_search.rs:34` — lexical search passes the
    raw query through: `self.store.search(query, limit, scope)`
  - `crates/domain/src/fusion.rs:132` — `free_text_to_fts5` (the neutralizer)
  - `crates/app/src/run.rs:274` — the CLI hands `args.query` to the lexical use
    case unmodified
  - `crates/adapters/store-sqlite/src/lib.rs:900` — the raw `query` is bound to
    `passages MATCH ?1`

### Observation

`mdsearch search "foo AND bar"` submits `foo AND bar` verbatim into the FTS5
`MATCH` clause, so FTS5 interprets `AND` as a boolean operator and unquoted
tokens/`prefix*`/quotes as FTS5 syntax. `mdsearch hybrid "foo AND bar"`
meanwhile routes through `free_text_to_fts5`, which wraps each whitespace term
in double quotes (escaping embedded quotes by doubling) and `AND`-joins them, so
the same string is treated as literal text that must all be present.

The domain already owns and unit-tests the neutralization behavior
(`crates/domain/src/fusion.rs:280-312`, `neutralizes_fts5_operator_characters`,
`collapses_whitespace`, `no_terms_maps_to_none`), and the lexical store already
has an error path for malformed FTS5 syntax (`SearchStoreError::InvalidQuery`,
see OBS-003). The asymmetry is therefore most plausibly an oversight: the
lexical path predates the free-text mapping introduced for hybrid search.

### Impact

- Same query, two different result semantics depending on command — surprising
  and hard to document.
- Lexical search accepts FTS5 operator syntax with no quoting, which is
  undocumented power-user behavior; a user typing a literal `*` or `OR` gets
  operator semantics (or an `InvalidQuery` error) instead of a literal match.
- Results differ between `search` and `hybrid` for the identical query string,
  which complicates evaluation (ADR-004 golden set) and agent-harness callers
  that switch between the two commands.

### Open questions (owner decision required)

1. Is the desired behavior (a) **literal free-text semantics on both commands**
   (apply `free_text_to_fts5` to lexical search too), (b) **preserve raw FTS5
   syntax on lexical search** as a documented power feature and neutralize only
   on hybrid, or (c) **unify with an explicit opt-in** (e.g., a flag to pass a
   raw FTS5 expression)?
2. If (a): does `search` keep the `InvalidQuery` path at all, or does it become
   unreachable for normal input?
3. Does the change affect the ADR-004 evaluation fixtures (queries in
   `xtask/src/eval/`), and must the golden set be re-baselined?

### Candidate epic direction

"Unify query semantics across lexical and hybrid search" — single source of
truth for query-to-FTS5 mapping, documented operator behavior, updated golden
fixtures, and regression scenarios in `007-lexical-search` and
`011-hybrid-search` feature packets.

---

## OBS-002 — Embedding dimension is hard-coded to 384, narrowing the advertised `--model` surface

- **Kind:** Architecture constraint
- **Priority:** High (blocks a supported-looking capability)
- **Status:** `PROMOTED` — committed as EPIC-009 in `PRD-001` (US-015,
  `specs/015-embedding-dimensions`); resolved decision recorded as DEC-014
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:41` —
    `const EMBEDDING_DIMENSION: i64 = 384;`
  - `crates/adapters/store-sqlite/src/lib.rs:180-185` — the `embeddings`
    virtual table is created with `dim=384`
  - `crates/adapters/store-sqlite/src/lib.rs:1336-1346` — `rebuild` hard-fails
    with `embedding dimension mismatch` when vector length ≠ 384
  - `crates/adapters/embed-fastembed/src/embedding.rs:111-122` — `friendly_model`
    advertises `bge-large-en-v1.5` and `multilingual-e5-large`, which are
    1024-dimension models
  - `crates/app/src/cli.rs:100-111` — the CLI exposes a generic `--model`
    switch with no dimension restriction
  - `crates/application/src/embed_collections.rs:112` — default
    `all-MiniLM-L6-v2` (384-dim)

### Observation

The schema, the `EMBEDDING_DIMENSION` constant, and the dimension guard in
`rebuild` are all pinned to 384. The CLI and the adapter's friendly-name map
accept models whose embedding dimensionality differs from 384. Selecting such a
model is accepted by the CLI and resolved by `resolve_model`, but the very first
collection rebuild then fails at `lib.rs:1338-1346` with a storage-level
"dimension mismatch" error — a late, confusing failure that is not surfaced as a
model-selection problem.

### Impact

- The `--model` flag over-advertises: valid, supported-looking model names lead
  to a hard runtime failure instead of either working or being rejected up front.
- Dimension is not recorded in `semantic_index_state`, so a future dimension
  change (or a mixed-dimension database) has no metadata to detect against.
- Only one model class is actually usable today (384-dim), which constrains
  evaluation and quality work (ADR-004) and the success-metric targets in
  `PRD-001` §3.

### Open questions (owner decision required)

1. Is multi-dimensionality support (store dimension per index, create the
  vector table at the model's dimension, validate per collection) in scope for
  `v0.2.0`, or is the product constrained to one fixed dimension by decision?
2. If fixed at 384: should the CLI/adapter reject non-384 models with a clear
  error before any collection work (fail fast at `--model` parse / `ensure_available`)?
3. Should the model's dimension be recorded in `semantic_index_state` and
  validated on `status`/`hybrid` reads for future-proofing?

### Candidate epic direction

"Embedding model dimension support" — dimension-aware schema (dimension in the
vector table and index state), validation at selection time, documented model
matrix (name → dims), and error scenarios in `010-semantic-index` and
`011-hybrid-search`. If dimension support is explicitly out of scope, a smaller
slice: fail fast on unsupported-dimension models.

---

## OBS-003 — Invalid FTS5 query detection relies on fragile error-message matching

- **Kind:** Robustness / correctness
- **Priority:** Medium
- **Status:** `PROMOTED` — folded into EPIC-008 in `PRD-001` (US-014,
  `specs/014-unify-query-semantics`)
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:1613-1623` —
    `search_query_failure` classifies an error as `InvalidQuery` only when the
    SQLite error message string contains `"fts5"`

### Observation

`search_query_failure` inspects the SQLite error message text
(`message.contains("fts5")`) to distinguish a malformed FTS5 query from a
generic storage failure. This is string-matching on an engine's human-readable
message; if `rusqlite`/SQLite wording changes (or error codes are used
instead), the classification silently degrades and every query error becomes a
generic `Storage` error, losing the user-facing `InvalidQuery` message.

### Impact

- The `SearchError::InvalidQuery`/`SearchStoreError::InvalidQuery` contract
  (used by the CLI to explain a bad query) is only as reliable as the matched
  substring.
- Contributes to the inconsistency in OBS-001: today the only thing turning a
  syntactically bad FTS5 expression into a friendly error is this string match.

### Open questions

1. Should the query path stop relying on engine error text — e.g., pre-validate
  the query (OBS-001's neutralization removes most operator risk) or classify on
  `rusqlite::Error` variant / SQLite extended result code instead of message text?
2. Is a regression scenario for "malformed query yields `InvalidQuery`" present
  in the store integration tests, or only implicitly via the message heuristic?

### Candidate epic direction

Fold into the OBS-001 slice ("query semantics") as the error-reporting side of
the same epic: deterministic query validation and classification independent of
engine message text.

---

## OBS-004 — Passage position computation re-reads the full file content per result

- **Kind:** Scalability / performance
- **Priority:** Medium (PRD-scale collections)
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:1629-1650` — `compute_position`
    counts newlines by scanning `content[..offset]`
  - `crates/adapters/store-sqlite/src/lib.rs:892` — lexical `search_results`
    selects `f.content AS file_content` for **every** matching row
  - `crates/adapters/store-sqlite/src/lib.rs:932` — position computed per row
    from that content
  - `crates/adapters/store-sqlite/src/hybrid.rs:205,384` — same pattern in the
    hybrid lexical leg
  - `crates/adapters/store-sqlite/src/lib.rs:946-964` — `count_matches` runs a
    second full scan (`COUNT(*)`) per search

### Observation

To compute a result's `line_start`/`line_end`, the store fetches the entire
stored file content blob for each matching passage and scans the bytes up to the
passage offset counting `\n`. With `limit` results per search (default 10, up to
100) over files that may be "100–300 page documents" (PRD-001 §5), this is
repeated per-row full-content transfer + linear scan, plus a second aggregate
query for `total`. It is correct but grows with file size × result count.

### Impact

- Query latency scales with (matched rows) × (file size), which directly
  opposes the "fast enough for harness calls" soft target in PRD-001 §3/§5.
- The `count_matches` re-query doubles storage work per lexical search.
- At 5,000-doc collections with large files, the human and JSON output paths
  (which consume `Position`) carry this cost even when the caller ignores
  positions.

### Open questions

1. Is position fidelity required on every result, or could line ranges be
  computed lazily (only when rendering) / from a stored per-passage offset
  without re-reading the file body?
2. Should `byte_offset`/line metadata be derived at index time and stored in
  `passage_files` (the `byte_offset` column already exists), removing the
  content re-read entirely?
3. Is a second query acceptable for `total`, or should it be derived from the
  same scan?

### Candidate epic direction

"Index-time passage positions" — store line-start/line-end (and verify byte
offset) in `passage_files` at rebuild, read positions without file content in
search/hybrid legs, and benchmark query latency on PRD-scale fixtures before and
after. Optionally fold `count_matches` into the result query.

---

## OBS-005 — Store helpers are duplicated across the SQLite adapter modules

- **Kind:** Maintainability / DRY
- **Priority:** Low–Medium
- **Locations (repeated definitions):**
  - `resolve_collection_id` — `lib.rs:1541`, `hybrid.rs:338`,
    `graph.rs:146`, `lib.rs:1394` (semantic store), `lib.rs:1511`
    (retrieval store)
  - `index_is_built` — `lib.rs:979`, `hybrid.rs:351`
  - `schema_version` — defined `lib.rs:1533`, imported by `hybrid.rs`
  - error-conversion helpers (`*_storage_failure`, `database_unavailable`) —
    `lib.rs:1589-1623`, `hybrid.rs:398`, `graph.rs:160`

### Observation

Five store types each re-implement `resolve_collection_id` (same SQL, same
`OptionalExtension` + `query_row` shape), and the `index_is_built`/
`schema_version` and error-wrapping helpers are similarly copy-pasted across
`lib.rs`, `hybrid.rs`, and `graph.rs`. The query text itself is duplicated too
(e.g., the lexical leg SQL appears in `lib.rs:890` and `hybrid.rs:202` with
near-identical projection).

### Impact

- A schema change (e.g., OBS-002 adding dimension columns, or OBS-004 adding
  position columns) must be applied consistently in several places; a missed
  copy is a latent inconsistency bug.
- Error-mapping drift: the semantic store and retrieval store already wrap
  errors slightly differently (`semantic_storage_failure` vs
  `retrieval_storage_failure`), so behavior can diverge per store.

### Open questions

1. Is a shared internal helper module (e.g., `store-sqlite/src/util.rs` or a
  `SqliteStore` wrapper holding `Connection` + shared resolvers) worth the
  churn, or is the duplication acceptable at this size?
2. Should the shared SQL be factored into constants/functions to keep the
  lexical and hybrid legs provably identical?

### Candidate epic direction

"SQLite adapter consolidation" — internal shared helpers and SQL constants;
verify via the existing store integration tests that behavior is unchanged. Low
user-visible value; it is mainly a risk-reduction epic and can be parked if the
v0.2.0 epic list is full.

---

## OBS-006 — Graph store silently coerces unknown node kinds and relations

- **Kind:** Robustness / correctness
- **Priority:** Medium
- **Locations:**
  - `crates/adapters/store-sqlite/src/graph.rs:65` —
    `EntityKind::from_key(&kind).unwrap_or(EntityKind::File)` on node lookup
  - `crates/adapters/store-sqlite/src/graph.rs:135-137` — same fallback for
    node kind and `RelationKind::from_key(&relation).unwrap_or(RelationKind::LinksTo)`
  - Contrast: the lexical/semantic stores **error** on unknown passage kinds
    instead of falling back (`lib.rs:929-931`, `lib.rs:1297-1301`)

### Observation

When the `nodes`/`edges` tables contain a `node_kind` or `relation` string not
in the known set (schema drift, a bad migration, or hand-edited data), the graph
store returns a node/edge typed as `File`/`LinksTo` instead of reporting an
error. The `CHECK` constraints on `nodes.node_kind` and `edges.relation`
(`lib.rs:219,229`) prevent this under normal writes, but the store itself does
not defend against it, and the fallback silently changes the meaning of a query
(`graph neighbors`/`related`/`context`).

### Impact

- Corruption or drift produces wrong graph results with no error, which is worse
  than a loud failure for a retrieval tool whose value is grounded context.
- Inconsistent with the rest of the adapter, which treats unknown kind keys as
  storage errors.

### Open questions

1. Should `SqliteGraphStore::node`/`neighbors` map unknown kind/relation strings
  to `GraphStoreError::Storage` (matching the lexical/semantic stores) instead
  of defaulting?
2. Are there legacy databases (pre-`create_graph_tables`, ADR history) whose
  rows would legitimately trip the stricter behavior and need a migration step?

### Candidate epic direction

"Graph store strictness" — replace silent defaults with explicit storage errors,
add corruption/regression scenarios to `012-entity-graph` and `013-context-recovery`
store tests. Small, self-contained; pairs naturally with OBS-005 consolidation.

---

## OBS-007 — File retrieval by basename scans every stored path in Rust

- **Kind:** Scalability / performance
- **Priority:** Low
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:1481-1508` —
    `list_by_basename` loads all `(path, content)` rows for the collection and
    filters by basename in Rust
  - `crates/application/src/get_file.rs:45-67` — the use case calls
    `get_by_path`, then `list_by_basename`, and uses the count to disambiguate

### Observation

`mdsearch get <collection> <name>` resolves a name by (1) exact path lookup,
then (2) a full scan of every stored file in the collection to collect
basename matches — including fetching the `content` blob of each row even though
only `path` is needed. The `content` fetch is wasted work, and the scan is
O(collection size) per ambiguous-name lookup.

### Impact

- At the top of the PRD scale (5,000 docs) a `get` by ambiguous basename loads
  every file body from the DB just to compare names — avoidable I/O.
- Minor today, but it shares the same "query only what you need" theme as
  OBS-004 and would be cheap to fix while the store SQL is being consolidated.

### Open questions

1. Should `list_by_basename` select only `path` (drop `content`) and/or use SQL
  (`WHERE substr(path, -length(?)) = ?` or a `file_name` helper) instead of a
  Rust-side scan?
2. Does the product need to return file content for all basename matches, or is
  a path list sufficient for disambiguation (`GetFileError::Ambiguous` only uses
  paths, `lib.rs`/`get_file.rs`)?

### Candidate epic direction

Fold into the OBS-004/OBS-005 performance-consolidation epic: "retrieval
queries fetch only required columns" — select `path` only for basename
resolution, keep the disambiguation contract, and add a store regression test.

---

## OBS-008 — Destroying a collection leaves orphaned records in child and virtual tables

- **Kind:** Robustness / correctness
- **Priority:** Critical (data leakage, orphaned records)
- **Status:** `PROMOTED` — committed as EPIC-010 in `PRD-001` (US-016,
  `specs/016-destroy-integrity`); resolved decision recorded as DEC-015
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:314-331` — `destroy_collection`
    executes only `DELETE FROM collections WHERE name_key = ?1`
  - `crates/adapters/store-sqlite/src/lib.rs:130-185,215-242` — schema declares
    `REFERENCES collections(collection_id) ON DELETE CASCADE` on child tables,
    but SQLite default connection mode disables foreign key enforcement
  - `crates/adapters/store-sqlite/src/lib.rs:150-153,180-185` — virtual tables
    (`passages` FTS5 and `embeddings` sqlite-vector) cannot declare SQLite foreign
    keys and have no automated cascade

### Observation

`SqliteCollectionStore::destroy_collection` executes a single SQL statement:
`DELETE FROM collections WHERE name_key = ?1`. Because SQLite does not enable
foreign key enforcement by default on new database connections (`PRAGMA
foreign_keys = ON;` is not executed upon connection opening), the foreign key
cascade delete actions are not executed by SQLite. Furthermore, virtual tables
(`passages` and `embeddings`) cannot declare foreign keys in SQLite.

As a consequence, destroying a collection leaves its stored files in `files`,
full-text rows in `passages`, mapping rows in `passage_files`, vector rows in
`embeddings`, graph nodes in `nodes`, graph edges in `edges`, and state records
in `lexical_index_state`, `semantic_index_state`, and `graph_state` orphaned
permanently in the database.

### Impact

- Orphaned records permanently consume storage space in the SQLite database.
- Queries operating across all collections (`SearchScope::All`) or joining
  against passages/embeddings can match against orphaned data from destroyed
  collections.
- If a newly created collection happens to receive the same `collection_id` as a
  previously destroyed collection, it inherits the old collection's orphaned
  files, passages, embeddings, and graph nodes/edges.

### Open questions (owner decision required)

1. Should `destroy_collection` execute an explicit multi-table transactional
   delete (clearing `embeddings`, `passages` via `passage_files`, `passage_files`,
   `files`, `nodes`, `edges`, state tables, and `collections`) to guarantee
   complete cleanup?
2. Should `SqliteCollectionStore::open` and related opening helpers enable
   `PRAGMA foreign_keys = ON;` on all SQLite connections by default (see
   OBS-011)?

### Candidate epic direction

"Collection lifecycle data integrity" — complete transactional cascade deletion
of all child tables, virtual tables, and index states on `destroy_collection`,
with regression integration tests verifying database table emptiness after
destroy.

---

## OBS-009 — Knowledge graph extraction does not support wikilinks (`[[note]]` and `[[note|label]]`)

- **Kind:** Behavior inconsistency
- **Priority:** High (standard markdown vault feature)
- **Locations:**
  - `crates/domain/src/graph.rs:349,489-514` — `inline_markdown_links` searches
    only for standard markdown link syntax matching `](` with `.md` extension
  - `crates/domain/src/graph.rs:416-465` — `resolve_file` resolves relative
    paths, parent directory joins, and bare basename matches

### Observation

Knowledge graph extraction parses only standard Markdown links of the form
`[label](target.md)`. The primary target domain for `mdsearch` is "developer
markdown knowledge vaults" (such as Obsidian, Logseq, Foam, Dendron, and QMD
notes), where intra-vault cross-references are predominantly written using
wikilinks syntax: `[[target]]`, `[[target|label]]`, `[[path/target#heading]]`, or
`[[target#heading|label]]`. None of these wikilinks are recognized by
`inline_markdown_links`, resulting in zero `LINKS_TO` graph edges being
generated for wikilink-based vaults.

### Impact

- Entity graphs extracted from typical Obsidian/LLM-wiki vaults remain largely
  disconnected, missing `LINKS_TO` edges.
- `--related` flags and `graph neighbors` queries miss document relationships
  for vaults that rely on wikilinks.

### Open questions (owner decision required)

1. Should wikilink extraction be supported in addition to standard
   `[label](target.md)` markdown links in `extract_graph`?
2. How should piped aliases in wikilinks (e.g. `[[target|alias label]]`) be
   treated: as a `LINKS_TO` edge to `target` only, or should `alias label` also be
   registered as an `Alias` node with an `ALIAS_OF` edge?
3. Should header fragments in wikilinks (e.g. `[[target#Section Name]]`) be
   stripped to resolve the destination file, matching `strip_link_target`
   behavior?

### Candidate epic direction

"Wikilink graph extraction" — support `[[target]]`, `[[target|label]]`, and
`[[target#heading]]` in domain graph extraction, resolve them against known
collection files via `resolve_file`, and add scenario coverage to
`012-entity-graph`.

---

## OBS-010 — Code fence unaware paragraph splitting and inline link extraction

- **Kind:** Robustness / correctness
- **Priority:** Medium
- **Locations:**
  - `crates/domain/src/passage.rs:269-302` — `split_paragraphs` splits body text
    on any blank line (`line.trim().is_empty()`)
  - `crates/domain/src/graph.rs:489-514` — `inline_markdown_links` scans raw
    content bytes for `](` without checking for code fences

### Observation

1. `split_paragraphs` divides document bodies strictly by blank lines without
   tracking whether the lines occur inside a fenced code block (` ``` ` or
   `~~~`). A code snippet containing internal empty lines is chopped into
   multiple fragmented passages; subsequent chunks lose the enclosing code
   fence and language identifier context.
2. `inline_markdown_links` scans raw bytes for `](` without ignoring code fences
   or inline code spans (e.g., `` `[link](target.md)` `` or example markdown in a
   code block). If a file matching the example target exists in the collection,
   a phantom `LINKS_TO` edge is created.

### Impact

- Code snippets in technical vaults are fragmented across multiple search
  passages, degrading semantic retrieval quality and search display readability.
- Documentation vaults that mention markdown link syntax in code examples can
  generate false-positive graph connections.

### Open questions

1. Should `split_paragraphs` track fenced code blocks and keep an entire fenced
   block as a single passage even if it contains blank lines?
2. Should link extraction ignore text inside fenced code blocks and inline
   backticks?

### Candidate epic direction

"Code-fence aware passage segmentation and link extraction" — maintain fence
state (` ``` `, `~~~`) during paragraph splitting and link extraction,
preserving code block unity and preventing phantom edge creation.

---

## OBS-011 — SQLite database connections lack production PRAGMA configurations

- **Kind:** Scalability / performance
- **Priority:** Medium
- **Locations:**
  - `crates/adapters/store-sqlite/src/lib.rs:55-83` — `SqliteCollectionStore::open`
    and `open_existing`
  - `crates/adapters/store-sqlite/src/lib.rs:100-112` —
    `SqliteFileStore::open_for_ingestion`
  - `crates/adapters/store-sqlite/src/lib.rs:727-743` —
    `SqliteLexicalIndexStore::open`
  - `crates/adapters/store-sqlite/src/lib.rs:815-831` —
    `SqliteLexicalSearchStore::open`
  - `crates/adapters/store-sqlite/src/lib.rs:996-1017` —
    `SqliteSemanticIndexStore::open_for_embedding`
  - `crates/adapters/store-sqlite/src/lib.rs:1426-1442` —
    `SqliteFileRetrievalStore::open`
  - `crates/adapters/store-sqlite/src/graph.rs:17-33` — `SqliteGraphStore::open`
  - `crates/adapters/store-sqlite/src/hybrid.rs:21-39` —
    `SqliteHybridSearchStore::open`

### Observation

SQLite database connections across all adapter entry points are opened with
default SQLite engine settings. Specifically:
- `PRAGMA foreign_keys = ON;` is never executed (see OBS-008).
- `PRAGMA journal_mode = WAL;` is not enabled, leaving SQLite in rollback journal
  `DELETE` mode.
- `PRAGMA synchronous = NORMAL;` and `PRAGMA busy_timeout` are not configured.

### Impact

- Rollback journal mode requires exclusive database file locks and full
  synchronous disk flushes during indexing updates, which reduces write
  throughput and blocks concurrent CLI reads.
- Foreign key constraints declared in `CREATE TABLE` DDL are ignored at runtime.

### Open questions

1. Should standard SQLite production PRAGMAs (`foreign_keys = ON`,
   `journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`) be
   executed uniformly whenever a database connection is opened or initialized?
2. Does WAL mode introduce any multi-file artifact concerns for single-file
   database management (WAL and SHM sidecar files exist while connections are
   active)?

### Candidate epic direction

"SQLite connection PRAGMAs and concurrency configuration" — centralize
connection initialization to set WAL mode, synchronous=NORMAL, foreign keys,
and busy timeout on all open paths.

---

## OBS-012 — Evaluation dataset scale is smaller than the target in ADR-004

- **Kind:** Maintainability / DRY
- **Priority:** Low–Medium
- **Locations:**
  - `xtask/src/eval/mod.rs` — evaluation harness runner
  - `tests/fixtures/eval/corpus.jsonl` — test corpus (32 documents)
  - `tests/fixtures/eval/queries.jsonl` — test queries (32 queries)
  - `tests/fixtures/eval/qrels.jsonl` — relevance judgments (78 judgments)
  - `specs/adr/ADR-004.md` — Golden dataset strategy specification

### Observation

`ADR-004` outlines a comprehensive golden dataset strategy with a 100–300 query
suite spanning curated queries, synthetic queries, and hard negatives over a
representative vault corpus (100–5,000 document scale). The current evaluation
dataset in `tests/fixtures/eval/` is a 32-document, 32-query fixture. While the
suite passes with 100% metrics (Recall@5 = 1.0, MRR@5 = 1.0, NDCG@5 = 1.0),
the small corpus size does not sufficiently stress-test ranking discrimination,
RRF fusion parameter sensitivity ($k=60$), or cross-encoder re-ranker
effectiveness under real vault conditions.

### Impact

- Perfect 1.0 scores on the minimal 32-query set may mask subtle ranking
  regressions or suboptimal fusion weights that would appear on larger, noisier
  collections.

### Open questions

1. When should the evaluation corpus be expanded to the 100–300 query scale
   specified in ADR-004?
2. Should synthetic query generation tooling be included in `xtask` to assist in
   generating and validating large query sets?

### Candidate epic direction

"Evaluation fixture expansion" — populate `tests/fixtures/eval/` with the full
100–300 query suite, hard negatives, and graded relevance judgments per ADR-004.

---

## OBS-013 — Ephemeral Tokio runtime creation in the synchronous `context` CLI command

- **Kind:** Maintainability / DRY
- **Priority:** Low
- **Locations:**
  - `crates/app/src/run.rs:431-434` — `tokio::runtime::Builder::new_current_thread().build()`
    inside `context` command
  - `crates/app/src/graph_query.rs:44,70` — async GraphQL resolvers with
    `#[allow(clippy::unused_async)]`

### Observation

`mdsearch context '<query>'` builds a new single-threaded Tokio runtime on each
invocation to execute the `async_graphql` schema via
`runtime.block_on(schema.execute(query))`. However, the underlying GraphQL
resolvers in `crates/app/src/graph_query.rs` are entirely synchronous
mutex-guarded calls against `SqliteGraphStore` and contain no actual
asynchronous I/O (requiring `#[allow(clippy::unused_async)]`).

### Impact

- Minor runtime bootstrapping overhead per `context` invocation.
- Slight architectural complexity from running a synchronous store through an
  asynchronous executor.

### Open questions

1. Should `async-graphql`'s in-process execution remain async for future
   extensibility (e.g. if async graph loaders or streaming are added), or be
   simplified?

### Candidate epic direction

"GraphQL query layer cleanup" — document runtime semantics and retain async
execution for future compatibility.

---

## Provisional Epic Mapping (for v0.2.0 planning)

| OBS | Candidate epic (working title) | Kind | Suggested scope for v0.2.0 |
| --- | --- | --- | --- |
| OBS-001 | Unify query semantics across lexical and hybrid search | Product-visible behavior | `PROMOTED` to EPIC-008 (US-014) |
| OBS-002 | Embedding model dimension support (or fail-fast on unsupported dims) | Product-visible behavior + schema | `PROMOTED` to EPIC-009 (US-015) |
| OBS-003 | Deterministic FTS5 query validation and error classification | Robustness | `PROMOTED` — folded into EPIC-008 (US-014) |
| OBS-004 | Index-time passage positions (drop per-result file re-read) | Performance | Candidate; validate against latency targets (PRD-001 §3/§5) |
| OBS-005 | SQLite adapter consolidation (shared helpers/SQL) | Maintainability | Candidate; parkable if capacity is tight |
| OBS-006 | Graph store strictness (no silent kind/relation fallback) | Robustness | Small; natural pairing with OBS-005 |
| OBS-007 | Retrieval queries fetch only required columns | Performance | Fold into OBS-004/OBS-005 |
| OBS-008 | Collection lifecycle data integrity (cascade destroy child/virtual tables) | Robustness / correctness | `PROMOTED` to EPIC-010 (US-016) |
| OBS-009 | Wikilink graph extraction (`[[note]]` and `[[note|label]]`) | Behavior inconsistency | High priority; standard markdown vault support |
| OBS-010 | Code-fence aware passage segmentation and link extraction | Robustness / correctness | Candidate; preserves code block integrity |
| OBS-011 | SQLite connection PRAGMAs and concurrency configuration | Scalability / performance | Candidate; pair with OBS-008 and OBS-005 |
| OBS-012 | Evaluation fixture expansion (100–300 query suite per ADR-004) | Maintainability / DRY | Parkable / follow-up release |
| OBS-013 | GraphQL query layer cleanup | Maintainability / DRY | Low priority / cleanup |

## Decisions Still Open

1. **PRD vehicle for v0.2.0** — update `PRD-001` (default per
   `specs/prd_lifecycle_and_evolution_plan.md` §4.1) vs. a new
   `scope: major-feature` or `scope: project` PRD. The lifecycle plan's rule of
   thumb: incremental epics within the existing product boundary → update
   `PRD-001`; a large multi-epic sub-system with its own personas/journeys →
   new PRD. None of the entries above obviously introduces new personas or a
   product pivot, which leans toward an `PRD-001` update, but the owner decides.
2. **Epic selection** — OBS-001 (with OBS-003 folded in) is committed as
   EPIC-008 / US-014, OBS-002 as EPIC-009 / US-015, and OBS-008 as
   EPIC-010 / US-016; the remaining observations await epic selection. The
   provisional mapping above is a suggestion only.
3. **Severity confirmation** — each entry records a proposed priority; confirm
   or adjust during epic refinement.

