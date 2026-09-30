# mdsearch

**Local markdown knowledge search** — a single-binary, offline CLI that turns a
developer's markdown vault into a queryable database with three index types:
lexical (BM25/FTS5), semantic (vector embeddings), and an entity graph of
files, tags, and aliases.

`mdsearch` retrieves only. It does not generate answers — you feed its ranked,
grounded passages and full files to an LLM or harness for synthesis. Everything
runs locally, fully offline, with no server, no network dependency, and no LLM
required at runtime.

- **Local-first and offline by default.** All indexing and search works with no
  network. External model downloads are opt-in (`--download`).
- **One embedded database file per machine.** Collections, files, lexical and
  semantic indexes, and the entity graph all live in
  `~/.mdsearch/collections.db` (overridable per command with `--database PATH`).
- **Explicit indexing.** `mdsearch update` re-indexes when files are added,
  modified, or deleted. There is no file watching.
- **Human-readable output by default.** Richer machine-readable JSON is opt-in
  via `--json`.
- **Retrieval-only.** No answer generation; the tool grounds context for an LLM
  harness.

---

## Table of Contents

- [Concepts](#concepts)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Command reference](#command-reference)
- [Migration from the previous CLI](#migration-from-the-previous-cli)
- [Entity graph](#entity-graph)
- [JSON output](#json-output)
- [Frontmatter reference](#frontmatter-reference)
- [Indexing model](#indexing-model)
- [Models and external services](#models-and-external-services)
- [Scope and boundaries](#scope-and-boundaries)
- [Development](#development)
- [Repository layout](#repository-layout)

---

## Concepts

### Collections and the database

A **collection** is a named group of markdown files. All collections for a
machine live in one SQLite database file, by default
`~/.mdsearch/collections.db`. Every command accepts `--database PATH` to address
a different database file (for example, per-project vaults).

### Index types

| Index | Built by | Purpose |
| --- | --- | --- |
| **Lexical** (FTS5/BM25) | `update` | Ranked passage search over file content and frontmatter. |
| **Semantic** (vector embeddings) | `update` (when enabled) | Passage-level semantic search; powers `search --mode hybrid`. |
| **Entity graph** (nodes + edges) | `update` | Files, tags, and aliases as nodes with typed relationships; powers `--related`, `graph neighbors`, and `graph query`. |

### Frontmatter

Each file's YAML frontmatter block is parsed for `title`, `tags`, `aliases`, and
`summary` (indexed as searchable passages) and for `related`/`sources`
(graph-only). See [Frontmatter reference](#frontmatter-reference).

### Retrieval surface

- `search` — lexical passages by default; `--mode hybrid` fuses lexical and semantic results.
- `get` — retrieve a complete stored file by name or indexing-assigned ID.
- `--related` on `search` in either mode — file-to-file related links per result.
- `graph neighbors` / `graph query` — inspect or query the entity graph.

---

## Installation

### From source

```sh
git clone https://github.com/qlambeau/mdsearch.git
cd mdsearch
cargo build --release --bin mdsearch
```

The single binary is produced at `target/release/mdsearch`. Copy or symlink it
onto your `PATH`:

```sh
install -m 0755 target/release/mdsearch ~/.local/bin/mdsearch
```

> **Note:** the Rust crate that contains the binary is internally named
> `kv-app` (a legacy "kv" working title); the produced binary is `mdsearch`.
> You only need `--bin mdsearch`, never the internal crate name.

### Requirements

- Rust toolchain (see `rust-toolchain.toml`; the project uses Rust 2024 edition).
- `sqlite-vector` is statically linked; no system SQLite or vector library is
  needed.
- Model assets (for semantic updates and hybrid search) are downloaded on demand with
  `--download`; everything else works offline with no downloads.

### Verify

```sh
mdsearch --version
mdsearch --help
```

---

## Quick start

Register a vault as a source, index it, and search:

```sh
# 1. Register a collection source (registration does not index files).
mdsearch collection create Notes ~/vault

# 2. Build the lexical index and entity graph.
mdsearch update --collection Notes

# 3. Search lexically.
mdsearch search rust --collection Notes

# 4. See related files behind each result.
mdsearch search rust --collection Notes --related

# 5. Retrieve a complete file by name or by the file_id shown in results.
mdsearch get rust.md --collection Notes
mdsearch get --id 1 --collection Notes

# 6. Inspect stored-content freshness.
mdsearch status --collection Notes
```

Illustrative `search` output (stored paths are canonical absolute paths):

```text
1. /home/me/vault/sub/borrowing.md:3-3 (tags, score 0.272) [Notes, file_id 3]
rust
2. /home/me/vault/rust.md:2-2 (title, score 0.227) [Notes, file_id 1]
Rust Notes
3. /home/me/vault/rust.md:3-3 (tags, score 0.227) [Notes, file_id 1]
rust systems
4. /home/me/vault/rust.md:6-7 (body, score 0.105) [Notes, file_id 1]
# Rust
Ownership is key. See [Borrowing](sub/borrowing.md).
4 match(es)
```

### Enable semantic search

Enable semantic indexing for the collection, then update. The first update can
fetch the selected embedding model with explicit download permission:

```sh
mdsearch collection configure Notes --semantic on
mdsearch update --collection Notes --download
mdsearch search "borrowing rules" --mode hybrid --collection Notes --no-rerank
```

To enable the default cross-encoder reranker, download it explicitly:

```sh
mdsearch model set --reranker bge-reranker-base --download
mdsearch search "borrowing rules" --mode hybrid --collection Notes
```

Subsequent updates and searches use cached assets offline. To create a new
collection with semantic indexing enabled from the start, use
`collection create NAME PATH... --semantic`.

### Maintain a collection

After adding, changing, or deleting Markdown files, run `update -c Notes`.
Use `update --all` to refresh every collection. To move or replace a vault,
register the complete replacement source set first:

```sh
mdsearch collection configure Notes --sources ~/new-vault ~/reference.md
mdsearch update -c Notes
```

Source replacement takes effect on the next successful update, including
removing stored files that no longer belong to a registered source.

---

## Command reference

| Command | Purpose |
| --- | --- |
| `collection create NAME PATH...` | Register sources; optionally enable semantic indexing with `--semantic`. |
| `collection configure NAME` | Replace sources with `--sources` and/or change `--semantic on|off`. |
| `collection list` | List collections, sources, and enabled indexes; supports `--json`. |
| `collection delete NAME` | Delete stored collection data while retaining original files. |
| `update -c NAME` / `update --all` | Refresh configured indexes from registered sources. |
| `search QUERY...` | Retrieve lexical passages, or select `--mode hybrid`. |
| `get PATH_OR_NAME -c NAME` / `get --id ID -c NAME` | Write exact stored file bytes. |
| `status [-c NAME]` | Inspect index readiness, freshness, models, and build times. |
| `model list` / `model set` | Inspect supported assets or change database-wide models. |
| `graph neighbors KEY -c NAME` | Inspect outgoing graph relationships. |
| `graph query QUERY -c NAME` | Execute a collection-bound GraphQL query. |

### Global options

Every command accepts `-h/--help`. Commands that read or write the database
accept the global `--database PATH` option before, between, or after the command
and its arguments; the default is `~/.mdsearch/collections.db`. Use `-c` as a
short form of `--collection` and `-n` as a short form of `--limit` where those
switches apply. Help and version write to stdout and exit 0. Argument errors
write to stderr and exit 2; operational failures write to stderr and exit 1.
Commands that need a default path require `HOME`; help and version do not.
Use `mdsearch --version` to print the binary version, and append `--help` to
any command to inspect its arguments.

These invocations select the same database:

```sh
mdsearch --database ./notes.db status -c Notes
mdsearch status --database ./notes.db -c Notes
mdsearch status -c Notes --database ./notes.db
```

An explicit database path permits lexical updates, lexical search, file
retrieval, graph inspection, collection listing, and status without `HOME`. Operations needing model assets also need a resolvable cache directory:
`HF_HOME`, then `FASTEMBED_CACHE_DIR`, then `~/.mdsearch/models`. Lexical-only
hybrid search with `--no-rerank` does not need model assets.

### `collection create NAME PATH... [--semantic]`

Register one or more canonical Markdown file or directory sources without
indexing. Semantic indexing is disabled by default. Names are compared
case-insensitively; equivalent names cannot be created twice.

```sh
mdsearch collection create Notes ~/vault
mdsearch collection create Reference ~/reference.md --semantic
```

### `collection configure NAME [--sources PATH...] [--semantic on|off]`

Replace registered sources and/or change semantic policy; at least one option
is required. Configuration does not index content. The next successful update
deletes files stored only from removed sources. Disabling semantic indexing
removes that collection's vectors during its next successful update.

```sh
mdsearch collection configure Notes --sources ~/vault ~/reference.md
mdsearch collection configure Notes --semantic on
```

### `collection list [--json]`

List canonical sources and enabled lexical, graph, and semantic indexes. JSON
retains `name` and `sources` and adds an `indexes` object of enablement booleans.

```sh
mdsearch collection list
mdsearch collection list --json
```

### `collection delete NAME`

Delete a collection's stored files and all indexes atomically. Original source
files remain on disk.

```sh
mdsearch collection delete Notes
```

### `update (--collection NAME | --all) [--download] [--skip-unreadable] [--json]`

Exactly one scope is required. Discover additions, modifications, and deletions
across registered sources, deduplicate overlapping sources, and refresh lexical,
graph, and enabled semantic indexes. Files and configured indexes commit
atomically per collection. Any failure preserves that collection's previous
state; `--all` continues with other collections and exits 1 if any fail.

`--skip-unreadable` skips unreadable files while retaining their stored content.
An inaccessible source directory aborts the collection update. `--download`
authorizes missing model downloads; lexical-only updates require no models.
Legacy collections must register sources before updating:

```sh
mdsearch collection configure Legacy --sources ~/vault
mdsearch update --collection Legacy
mdsearch update --all --json
```

JSON is one complete document containing `collections`, each with `name` and
`success`. Successful entries report `added`, `modified`, `deleted`, `skipped`,
and `malformed_frontmatter`; failed entries report `diagnostic`. Success goes to
stdout. If any collection fails, the complete report goes to stderr, stdout is
empty, and the process exits 1.

### `search QUERY... [--mode lexical|hybrid]`

Use lexical BM25 retrieval by default. Hybrid retrieval fuses lexical and semantic
results and optionally reranks them with a local cross-encoder. Quoted queries
and separate words are equivalent literal free text, including operator-looking
characters. Empty or whitespace-only queries are rejected.

| Option | Description |
| --- | --- |
| `--mode lexical|hybrid` | Retrieval mode; default lexical. |
| `-c, --collection NAME` | Restrict to one collection; default all built collections. |
| `-n, --limit N` | Maximum results, 1–100; default 10. |
| `--json` | Preserve mode-specific JSON fields and include mode/file identity. |
| `--related` | Include related file links without changing ranking. |
| `--no-rerank` | Disable reranking in hybrid mode; invalid in lexical mode. |

```sh
mdsearch search borrowing rules -c Notes -n 5
mdsearch search "borrowing rules" --mode hybrid -c Notes
mdsearch search rust --mode hybrid --json --related --no-rerank
```

Human results identify collection, file ID, path, passage kind, score, and
position. Empty searches explicitly print `no matches`. Hybrid search never
downloads assets; missing prerequisites include a recovery command. Collections
without semantic indexes can still contribute lexical passages. An uncached
reranker falls back to fused ranking with the established warning.

### `get PATH_OR_NAME --collection NAME` / `get --id ID --collection NAME`

Exactly one selector is required. Names resolve by exact stored path first,
then unique basename; ambiguous names report candidate paths. Numeric names
remain names. Only `--id` selects a positive stored file ID (1–9223372036854775807).
Use the `file_id` returned by search; IDs are scoped to the selected collection.

```sh
mdsearch get rust.md -c Notes
mdsearch get --id 3 -c Notes
mdsearch get 42 -c Notes
```

Output is exactly the stored bytes: no added newline, text conversion, or JSON
wrapper. Empty files emit no bytes; existing trailing newlines and non-UTF-8
content are preserved.

### `status [--collection NAME] [--json]`

Report enabled indexes, readiness, stored-content freshness, selected global
models, recorded semantic model/dimension, counts, and last successful builds.
JSON reports each index's `enabled`, `readiness`, `freshness`, and `built_at`
fields, plus applicable passage/node/edge counts and semantic model metadata.

| Readiness | Meaning |
| --- | --- |
| `disabled` | Semantic indexing is disabled for the collection. |
| `not_built` | Enabled index has no successful build; `built_at` and `freshness` are null. |
| `ready` | Built index matches stored content and its active model configuration. |
| `stale` | Built index differs from stored content or active model configuration. |

Stale indexes retain their previous successful-build times. `freshness` is
`current` or `stale` for built indexes; semantic `model_compatible` reports
model/dimension compatibility separately. Disabling semantic policy can retain
previous build metadata until the next successful update clears that index.

Freshness compares indexes with **stored content**, without scanning registered
sources. Pending filesystem edits do not make stored indexes stale; run update
to incorporate them. Inspection is read-only and does not migrate legacy data.

```sh
mdsearch status
mdsearch status -c Notes --json
```

### `model list [--json]` / `model set [NAME] [--reranker NAME] [--download]`

List supported models and local availability. Select database-wide models;
provide at least one model choice. Embedding changes atomically rebuild affected
semantic-enabled collections, including dimension changes. Reranker-only changes
preserve vectors. Failed rebuilds preserve previous settings and indexes.

```sh
mdsearch model list --json
mdsearch model set all-MiniLM-L6-v2 --download
mdsearch model set --reranker bge-reranker-base --download
```

Downloads require `--download`. Asset caches follow `HF_HOME`, then
`FASTEMBED_CACHE_DIR`, then `~/.mdsearch/models`.

### `graph neighbors KEY --collection NAME [--kind KIND] [--relation RELATION] [--depth N]`

Inspect the selected collection's exact node key. Kind defaults to `file` and
accepts `file`, `tag`, or `alias`; depth defaults to one hop and accepts 1–255.
Relations use the existing vocabulary below. Traversal follows outgoing edges;
tags and aliases normally have no outgoing neighbors because metadata edges
point from files to those nodes. Results identify neighbor kind, key, title,
relation, and depth.

```sh
mdsearch graph neighbors /home/me/vault/rust.md -c Notes
mdsearch graph neighbors rust -c Notes --kind tag --depth 2
mdsearch graph neighbors /home/me/vault/rust.md -c Notes --relation LINKS_TO
```

### `graph query QUERY --collection NAME`

Execute an in-process read-only GraphQL document. Every resolver is bound to
CLI-selected collection context, including aliases and multiple roots. Public
fields do not accept collection arguments; attempts to supply them fail query
validation without graph data.

```sh
mdsearch graph query '{ neighbors(kind: "file", key: "/home/me/vault/rust.md", maxHops: 2) { kind key title relation depth } }' -c Notes
```

| Field | Arguments | Returns |
| --- | --- | --- |
| `node` | `kind`, `key` | `kind`, `key`, `title`; unknown nodes fail. |
| `neighbors` | `kind`, `key`, optional `relation`, `maxHops` | Neighbor kind/key/title/relation/depth; unknown start nodes fail. |

GraphQL is internal; no server or network endpoint is exposed.

### Migration from the previous CLI

The previous commands are rejected with argument errors (exit 2).

| Previous invocation | Replacement |
| --- | --- |
| `collection create NAME` | `collection create NAME PATH...` |
| `collection add NAME PATH...` | `collection configure NAME --sources PATH...`, then `update -c NAME` (configure replaces the source set) |
| `collection update NAME [PATH...]` | Register sources, then `update -c NAME` |
| `collection update --collection NAME` | `update --collection NAME` |
| `collection update --all` | `update --all` |
| `collection destroy NAME` | `collection delete NAME` |
| `embed --collection NAME` | `collection configure NAME --semantic on`, then `update -c NAME` |
| `embed [--download]` | Enable semantic policy on intended collections, then `update --all [--download]` |
| `embed --model NAME [--reranker NAME]` | `model set NAME [--reranker NAME] [--download]` |
| `embed --reranker NAME` | `model set --reranker NAME [--download]` |
| `hybrid QUERY` | `search QUERY --mode hybrid` |
| `get COLLECTION NAME` | `get NAME -c COLLECTION` |
| `get COLLECTION ID` | `get --id ID -c COLLECTION` |
| `index status` | `status [--collection NAME] [--json]` |
| `graph neighbors KEY` | `graph neighbors KEY -c NAME [--kind KIND] [--depth N]` |
| `context QUERY --collection NAME` | `graph query QUERY -c NAME`; remove collection arguments from query fields |
| `--force` | `--skip-unreadable` skips read failures; it does not force reprocessing |

Existing collections and semantic enablement are preserved. Migration does not
guess source directories. Read commands never create a missing database or
migrate existing data; authorized mutation paths apply existing migrations.

---

## Entity graph

`mdsearch update` builds a deterministic entity graph for each collection:
**nodes** are files, tags, and aliases; **edges** are typed and directional,
derived from frontmatter and inline links. The build is a full deterministic
rebuild, so nodes/edges from deleted or renamed files disappear, and re-running
on unchanged files produces an identical graph.

### Nodes

| Kind | Identity | Derived from |
| --- | --- | --- |
| `file` | canonical file path | each stored markdown file |
| `tag` | exact normalized tag name | frontmatter `tags:` |
| `alias` | exact normalized alias name | frontmatter `aliases:` |

A tag and an alias with the same name remain distinct nodes.

### Edges

| Relation | Direction | Derived from |
| --- | --- | --- |
| `LINKS_TO` | file → file | inline relative `.md` links |
| `TAGGED_WITH` | file → tag | frontmatter `tags:` |
| `ALIAS_OF` | file → alias | frontmatter `aliases:` |
| `RELATED_TO` | file → file | frontmatter `related:` |
| `HAS_SOURCE` | file → file | frontmatter `sources:` |

Unresolved `related:`/`sources:` references and inline link targets that do not
match a stored file are skipped (no edge, no error).

### `--related`

`--related` on `search` in either mode lists each result's **file-to-file** related
links only (`LINKS_TO`, `RELATED_TO`, `HAS_SOURCE`); tags and aliases are
omitted. In human output each link is one line; in JSON output it is a
`related` field per result. Ranked results are never changed by `--related`.

```text
1. /home/me/vault/rust.md:6-7 (body, score 0.105) [Notes, file_id 1]
# Rust
Ownership is key. See [Borrowing](sub/borrowing.md).
related: /home/me/vault/sub/borrowing.md (LINKS_TO)
```

---

## JSON output

`--json` on `search` in either mode emits a richer machine-readable object. The `--related`
switch adds a `related` field to each result. Example `search` JSON:

```json
{
  "mode": "lexical",
  "query": "rust",
  "scope": "Notes",
  "limit": 10,
  "total": 1,
  "results": [
    {
      "collection": "Notes",
      "file_id": 3,
      "path": "/home/me/vault/sub/borrowing.md",
      "kind": "tags",
      "text": "rust",
      "score": 0.272,
      "position": {
        "byte_offset": 21,
        "byte_length": 4,
        "line_start": 3,
        "line_end": 3
      }
    }
  ]
}
```

With `--related`, each result gains:

```json
{
  "related": [
    { "path": "/home/me/vault/rust.md", "relation": "RELATED_TO" }
  ]
}
```

`search --mode hybrid --json` reports `reranked`, `rerank_warning`, and per-result
`reranker_score`, `fused_score`, `bm25_score`, `cosine_similarity`, and
`ordering_score`.

`get` returns the raw stored file content (no JSON mode).

### Collection and update reports

`collection list --json` returns a `collections` array:

```json
{
  "collections": [
    {
      "name": "Notes",
      "sources": ["/home/me/vault"],
      "indexes": { "lexical": true, "graph": true, "semantic": false }
    }
  ]
}
```

`update --all --json` returns one complete report. For example, a partial
failure writes this document to stderr and exits 1; successful collections
have already committed their updates:

```json
{
  "collections": [
    {
      "name": "Archive",
      "success": false,
      "diagnostic": "source directory is unavailable"
    },
    {
      "name": "Notes",
      "success": true,
      "added": 1,
      "modified": 2,
      "deleted": 0,
      "skipped": 0,
      "malformed_frontmatter": 0
    }
  ]
}
```

For scripts, check the exit status and read the report from the corresponding
stream. A fully successful update writes its JSON report to stdout.

---

## Frontmatter reference

A YAML `---`-delimited block at the top of each file is parsed. Malformed
frontmatter is tolerated (the body is still indexed) and reported in `update`
output.

| Field | Indexed as | Graph use |
| --- | --- | --- |
| `title` | passage (`title`) | file node title |
| `tags` | passage (`tags`) | tag nodes + `TAGGED_WITH` |
| `aliases` | passage (`aliases`) | alias nodes + `ALIAS_OF` |
| `summary` | passage (`summary`) | — |
| `related` | — | `RELATED_TO` edges |
| `sources` | — | `HAS_SOURCE` edges |

Scalar or inline list values are supported (e.g. `tags: rust` or
`tags: [rust, systems]`).

---

## Indexing model

- Collection create/configure register sources and policy without indexing.
- `update` reconciles the file set (adds/modifies/deletes) and
  rebuilds the **lexical index**, **entity graph**, and enabled **semantic
  index** for each updated collection. Unchanged files retain their
  stored content; successful rebuilds refresh index build metadata.
- Semantic indexing is opt-in per collection. Configuration changes take
  effect during the next successful update; turning it off clears that
  collection's vectors during the update.
- There is no file watching. Re-run `update` after changing files on
  disk.
- Existing databases migrate on authorized mutation paths; migration is
  idempotent and never rewrites stored file, lexical, or semantic data.

---

## Models and external services

- Embedding (semantic updates and hybrid search) uses `fastembed` locally. The default model is
  `all-MiniLM-L6-v2`. Model assets are downloaded with `--download` and then
  cached locally; all later runs are offline.
- Downloaded assets live in the model cache directory, resolved per run as:
  `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then `~/.mdsearch/models`. A model is
  considered downloaded when its completion marker exists in that directory,
  so semantic updates and hybrid search never re-download (or advise re-downloading) a model
  that is already present. Legacy downloads in an old working-directory
  `.fastembed_cache` are not reused; they are fetched once into the new
  location.
- A cross-encoder re-ranker can be selected with `model set --reranker NAME` for hybrid search;
  its assets follow the same cache location and marker rules.
- Lexical search, `get`, `graph`, status, and collection inspection require no models and no network.

---

## Scope and boundaries

- **Retrieval only.** No answer generation; the harness/LLM synthesizes.
- **No file watching.** Indexing is driven by explicit `update`.
- **Single binary, single database file.** No server, web UI, multi-user,
  authentication, cloud sync, or hosted collections.
- **External services are opt-in** via CLI switches (`--download` for models);
  local operation is the default.

---

## Development

```sh
# Format and lint (constitution gates)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Tests and coverage
cargo test --workspace
cargo llvm-cov --workspace --fail-under-lines 85

# Full CI gate: fmt, clippy, tests, docs, deny, coverage
cargo xtask ci

# Offline retrieval evaluation against the golden dataset
cargo xtask eval
```

Rust engineering rules, Definition of Done, and the required tooling gates are
normative in `specs/CONSTITUTION.md` — read it before editing Rust code.

---

## Repository layout

- `crates/` — Rust workspace: `domain`, `application`, `adapters`
  (`store-sqlite`, `embed-fastembed`), `infrastructure`, `app` (the `mdsearch`
  binary crate), plus `xtask` (automation).
- `specs/` — PRDs, ADRs, feature packets (`NNN-feature-slug/`), schema, and
  templates for the spec-first workflow. The single normative workflow and gates
  live in `specs/SDD_WORKFLOW.md`; the single normative Rust engineering rules live
  in `specs/CONSTITUTION.md`.
- `docs/SDD_WORKFLOW_KIT.md` — informational kit manual (non-normative orientation guide; defers to `specs/SDD_WORKFLOW.md`).
- `vendor/` — vendored `sqlite-vector-rs` dependency.

See `specs/SDD_WORKFLOW.md` (normative workflow) and `docs/SDD_WORKFLOW_KIT.md` (informational manual) for how product intent is turned
into implementation-ready feature specifications.
