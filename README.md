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
| **Lexical** (FTS5/BM25) | `collection update` | Ranked passage search over file content and frontmatter. |
| **Semantic** (vector embeddings) | `embed` | Passage-level semantic search; powers `hybrid`. |
| **Entity graph** (nodes + edges) | `collection update` | Files, tags, and aliases as nodes with typed relationships; powers `--related`, `graph neighbors`, and `context`. |

### Frontmatter

Each file's YAML frontmatter block is parsed for `title`, `tags`, `aliases`, and
`summary` (indexed as searchable passages) and for `related`/`sources`
(graph-only). See [Frontmatter reference](#frontmatter-reference).

### Retrieval surface

- `search` — lexical (BM25) ranked passages.
- `hybrid` — fused lexical + semantic results, optionally re-ranked.
- `get` — retrieve a complete stored file by name or indexing-assigned ID.
- `--related` on `search`/`hybrid` — file-to-file related links per result.
- `graph neighbors` / `context` — inspect or query the entity graph.

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
- Model assets (for `embed`/`hybrid`) are downloaded on demand with
  `--download`; everything else works offline with no downloads.

### Verify

```sh
mdsearch --version
mdsearch --help
```

---

## Quick start

Create a vault, add it to a collection, index it, and search:

```sh
# 1. Create a collection.
mdsearch collection create Notes

# 2. Add a directory of markdown files to it.
mdsearch collection add Notes ~/vault

# 3. Build the lexical index and entity graph.
mdsearch collection update Notes ~/vault

# 4. Search lexically.
mdsearch search rust --collection Notes

# 5. See the related files behind each result.
mdsearch search rust --collection Notes --related

# 6. Check index state.
mdsearch index status
```

Example `search` output:

```text
1. ~/vault/sub/borrowing.md:3-3 (tags, score 0.272)
rust
2. ~/vault/rust.md:2-2 (title, score 0.227)
Rust Notes
3. ~/vault/rust.md:3-3 (tags, score 0.227)
rust systems
4. ~/vault/rust.md:6-7 (body, score 0.105)
# Rust
Ownership is key. See [Borrowing](sub/borrowing.md).
4 match(es)
```

---

## Command reference

### Global options

Every command accepts `-h/--help`. Commands that read or write the database
accept `--database PATH`; the default is `~/.mdsearch/collections.db`.

### `collection create NAME`

Create a new empty collection.

| Option | Description |
| --- | --- |
| `--database PATH` | Database file to use. |

```sh
mdsearch collection create Notes
# created collection "Notes"
```

Names are normalized for comparison; creating an equivalent name again fails
with a duplicate error.

### `collection list`

List all collections.

```sh
mdsearch collection list
# Notes
```

### `collection destroy NAME`

Delete a collection and everything stored under it (files, passages, vectors,
graph). This is destructive and permanent.

```sh
mdsearch collection destroy Notes
# destroyed collection "Notes"
```

### `collection add NAME PATH...`

Ingest one or more files or directories into a collection without indexing.
Paths are read recursively; every `.md` file found is stored with its frontmatter
parsed.

| Option | Description |
| --- | --- |
| `--force` | Re-add files even if unchanged. |
| `--database PATH` | Database file to use. |

```sh
mdsearch collection add Notes ~/vault
# added 3 files to collection "Notes"
```

### `collection update [NAME] [PATH...]`

Re-index a collection (or all collections), reconciling files against the
filesystem. This is the main indexing command: it upserts added/modified files,
deletes removed ones, rebuilds the **lexical index**, and rebuilds the
**entity graph** for each updated collection in one transaction. It does not
build the semantic index (see [`embed`](#embed)).

| Option | Description |
| --- | --- |
| `--all` | Update every collection in the database. |
| `--force` | Treat all stored files as modified. |
| `--database PATH` | Database file to use. |

`NAME` and `PATH...` are mutually exclusive with `--all`.

```sh
mdsearch collection update Notes ~/vault
# updated collection "Notes": added 0, modified 0, deleted 0
```

The update is transactional per collection: if indexing fails, the collection's
previous file/lexical/graph state is preserved.

### `index status`

Report lexical index state for each collection (file count, passage count, and
last build time).

```sh
mdsearch index status
# collection "Notes": lexical index built, 3 file(s), 9 passage(s), built at 1787178549
```

### `search QUERY`

Lexical (BM25) ranked passage search across one collection or all collections.

| Option | Description |
| --- | --- |
| `--collection NAME` | Restrict to one collection (default: all). |
| `--limit N` | Maximum results (1–100, default 10). |
| `--json` | Machine-readable JSON output. |
| `--related` | Add file-to-file related links per result (see [Entity graph](#entity-graph)). |
| `--database PATH` | Database file to use. |

```sh
mdsearch search rust --collection Notes --limit 5
mdsearch search "memory safety" --json
mdsearch search rust --collection Notes --related
```

Results show the file path, the passage kind (`title`, `tags`, `aliases`,
`summary`, or `body`), a score, and the matched passage text. An empty or
whitespace-only query is rejected.

### `get COLLECTION NAME_OR_ID`

Retrieve a complete stored file by its name or its indexing-assigned ID.

```sh
mdsearch get Notes rust.md
mdsearch get Notes 3
```

When the name is ambiguous (more than one file shares the basename), the command
reports the candidate paths.

### `embed`

Build the semantic (vector) index for one or all collections, optionally
selecting the embedding model and re-ranker.

| Option | Description |
| --- | --- |
| `--collection NAME` | Restrict to one collection (default: all). |
| `--model NAME` | Embedding model (default `all-MiniLM-L6-v2`). |
| `--reranker NAME` | Cross-encoder re-ranker model. |
| `--download` | Fetch model assets (required the first time). |
| `--database PATH` | Database file to use. |

```sh
mdsearch embed --collection Notes --download
mdsearch embed
```

Without a cached model and `--download`, embedding fails with a clear message:

```text
embedding model all-MiniLM-L6-v2 is not available locally; pass --download to fetch it
```

Model assets fetched with `--download` are stored under
`~/.mdsearch/models` by default (see
[Models and external services](#models-and-external-services)); a model counts
as downloaded once its completion marker exists there, regardless of the
working directory you run `mdsearch` from.

Embedding is skipped for collections with no files or no lexical index, and
skipped when the index is already current for the file set.

### `hybrid QUERY`

Hybrid search: fuse lexical (BM25) and semantic (cosine) scores into one ranked
list, optionally re-ranking with a cross-encoder.

| Option | Description |
| --- | --- |
| `--collection NAME` | Restrict to one collection (default: all). |
| `--limit N` | Maximum results (1–100, default 10). |
| `--json` | Machine-readable JSON output. |
| `--related` | Add file-to-file related links per result. |
| `--no-rerank` | Skip cross-encoder re-ranking. |
| `--database PATH` | Database file to use. |

```sh
mdsearch hybrid "borrowing rules" --collection Notes
mdsearch hybrid rust --json --related --no-rerank
```

If the semantic index is stale (files changed since `embed`), hybrid reports that
you should run `mdsearch embed`. If the re-ranker model is not cached, a warning
is printed; pass `--no-rerank` to suppress it.

### `graph neighbors ID`

Debug inspection of a node's neighbors in the entity graph, with relation types
and traversal depths (read-only).

| Option | Description |
| --- | --- |
| `--collection NAME` | Restrict to one collection (default: search all). |
| `--database PATH` | Database file to use. |

```sh
mdsearch graph neighbors ~/vault/rust.md --collection Notes
```

See [Entity graph](#entity-graph) for the node and relation vocabulary.

### `context '<graphql query>'`

Execute an in-process GraphQL query over the entity graph and print the JSON
result. The query is passed as a single positional argument. Read-only.

| Option | Description |
| --- | --- |
| `--collection NAME` | **Required.** The collection the query runs against. |
| `--database PATH` | Database file to use. |

```sh
mdsearch context '{ neighbors(collection: "Notes", kind: "file", key: "~/vault/rust.md", maxHops: 2) { key relation depth } }' --collection Notes
```

The exposed schema mirrors the internal query layer:

| Query | Arguments | Returns |
| --- | --- | --- |
| `node(collection, kind, key)` | collection name, node kind (`file`/`tag`/`alias`), node key | The node's `kind`, `key`, `title`; errors if the node does not exist. |
| `neighbors(collection, kind, key, relation?, maxHops)` | as above, optional relation filter (`LINKS_TO`, `TAGGED_WITH`, `ALIAS_OF`, `RELATED_TO`, `HAS_SOURCE`), hop limit | `[{ key, relation, depth }]`; errors if the start node does not exist. |

GraphQL remains in-process: no server or network endpoint is exposed.

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

`--related` on `search`/`hybrid` lists each result's **file-to-file** related
links only (`LINKS_TO`, `RELATED_TO`, `HAS_SOURCE`); tags and aliases are
omitted. In human output each link is one line; in JSON output it is a
`related` field per result. Ranked results are never changed by `--related`.

```text
1. ~/vault/rust.md:6-7 (body, score 0.105)
# Rust
Ownership is key. See [Borrowing](sub/borrowing.md).
related: ~/vault/sub/borrowing.md (LINKS_TO)
```

---

## JSON output

`--json` on `search`/`hybrid` emits a richer machine-readable object. The `--related`
switch adds a `related` field to each result. Example `search` JSON:

```json
{
  "query": "rust",
  "scope": "Notes",
  "limit": 10,
  "total": 4,
  "results": [
    {
      "collection": "Notes",
      "path": "~/vault/sub/borrowing.md",
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
    { "path": "~/vault/rust.md", "relation": "RELATED_TO" }
  ]
}
```

`hybrid --json` additionally reports `reranked`, `rerank_warning`, and per-result
`reranker_score`, `fused_score`, `bm25_score`, `cosine_similarity`, and
`ordering_score`.

`get` returns the raw stored file content (no JSON mode).

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

- `collection add` stores files; nothing is indexed yet.
- `collection update` reconciles the file set (adds/modifies/deletes) and
  rebuilds the **lexical index** and **entity graph** for each updated
  collection. It is deterministic and idempotent: re-running on unchanged files
  changes nothing.
- `embed` builds the **semantic index** from the stored files/passages; it is
  skipped when already current for the file set.
- There is no file watching. Re-run `update` (and `embed` when semantic results
  matter) after changing files on disk.
- Existing databases migrate forward automatically when opened; migration is
  idempotent and never rewrites stored file, lexical, or semantic data.

---

## Models and external services

- Embedding (`embed`, `hybrid`) uses `fastembed` locally. The default model is
  `all-MiniLM-L6-v2`. Model assets are downloaded with `--download` and then
  cached locally; all later runs are offline.
- Downloaded assets live in the model cache directory, resolved per run as:
  `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then `~/.mdsearch/models`. A model is
  considered downloaded when its completion marker exists in that directory,
  so `embed`/`hybrid` never re-download (or advise re-downloading) a model
  that is already present. Legacy downloads in an old working-directory
  `.fastembed_cache` are not reused; they are fetched once into the new
  location.
- A cross-encoder re-ranker can be selected with `--reranker NAME` for `hybrid`;
  its assets follow the same cache location and marker rules.
- Everything else (`search`, `get`, `graph`, `context`, index/collection
  lifecycle) requires no models and no network.

---

## Scope and boundaries

- **Retrieval only.** No answer generation; the harness/LLM synthesizes.
- **No file watching.** Indexing is driven by explicit `update`/`embed`.
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
  templates for the spec-first workflow.
- `docs/SDD_WORKFLOW_KIT.md` — the spec-first workflow kit documentation.
- `vendor/` — vendored `sqlite-vector-rs` dependency.

See `docs/SDD_WORKFLOW_KIT.md` and `specs/` for how product intent is turned
into implementation-ready feature specifications.
