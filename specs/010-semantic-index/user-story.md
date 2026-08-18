---
id: US-010
title: "Build the semantic index for collections"
type: user-story
status: implemented
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: PRD-001
epic: EPIC-004
feature: 010-semantic-index
related: []
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want to build a semantic (vector) index for my collections with an explicit `embed` command,
so that conceptual, non-keyword queries can later be matched against my markdown passages.

## Context And Value

`US-006` builds the per-passage lexical index during `collection update`, but the
engine currently has no semantic index. EPIC-004 adds semantic indexing and
hybrid search in two slices; this story delivers the first: a dedicated
`mdsearch embed` command that builds and maintains a per-passage vector index.
The embedding model is resolved during design (the story stays model-agnostic,
per the approved EPIC-004 slicing). Once this index exists, the next slice adds
the hybrid search command that fuses lexical and semantic results.

Both personas benefit: the developer-curator gets a maintainable semantic index
with a clear per-collection status, and the harness gets a foundation for
conceptual retrieval that plain keyword search cannot satisfy.

## Business Rules

- The command is `mdsearch embed`, with optional `--collection NAME`,
  `--model NAME`, `--download`, and `--database PATH`.
- Without `--collection`, every collection is processed; with `--collection
  NAME`, only that collection is processed.
- The semantic index embeds the same per-passage units the lexical index covers:
  body paragraphs plus the four frontmatter fields (`title`, `tags`, `aliases`,
  `summary`). Each vector is keyed to the same passage row the lexical index
  uses, so semantic and lexical retrieval return identical passages.
- Embedding requires a built lexical index:
  - In all-collections mode, a collection whose lexical index is not built is
    skipped silently and reported in the summary.
  - When `--collection` targets a collection whose lexical index is not built,
    the command fails and reports the index is not built.
- A collection with no stored files is skipped silently and reported in the
  summary, whether targeted via `--collection` or processed in all-collections
  mode.
- When `--collection` names a collection that does not exist, the command fails
  and reports the collection was not found.
- The semantic index is rebuilt per collection only when the stored file set
  (content hashes) or the global embedding model changed since the last embed.
  If neither changed, the command reports the collection's semantic index is
  already current and does not rebuild it.
- Each collection's rebuild is atomic: the old vectors are replaced with the new
  ones in one all-or-nothing transaction. A failed rebuild leaves the previous
  semantic index intact.
- There is a single global embedding model recorded in the database:
  - The first `embed` run records the model used (from `--model` or the
    default).
  - If a later `embed` passes `--model` that differs from the recorded model,
    the command switches the global model and rebuilds every collection that has
    an existing semantic index under the new model, regardless of the
    `--collection` scope of the invocation.
- `--model` validation is delegated to the embedding library; an unsupported
  model name fails with a clear error before any collection work.
- The embedding model's assets must be available locally:
  - Without `--download`, if the model is not cached locally, the command fails
    before touching any collection, with a clear error naming the model and
    suggesting `--download`.
  - With `--download`, the command fetches the model assets and then embeds in
    the same run.
  - If `--download` fails to fetch the assets, the command fails cleanly and no
    collection is modified.
- In all-collections mode, a failure to embed one collection is reported and
  processing continues with the remaining collections; the command's exit status
  reflects that a failure occurred.
- The output is a per-collection summary reporting, for each collection:
  embedded passage count, already current, skipped (no files or unbuilt lexical
  index), or failed, plus the model used.
- If the selected database does not exist, the command fails and reports the
  database does not exist without creating a file.
- Exact human-readable wording of errors and summary formatting is not part of
  this story's contract.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | `Notes` has files and a built lexical index, and no semantic index exists yet | I run `mdsearch embed` | `Notes` is embedded: summary reports the number of passages embedded and the model used |
| EX-002 | `Notes` was just embedded and no files or model changed | I run `mdsearch embed` again | `Notes` is reported already current and not rebuilt |
| EX-003 | A file in `Notes` was added or changed and `update` was run | I run `mdsearch embed` | `Notes` is rebuilt with the current passage set |
| EX-004 | `Notes` is embedded under model A | I run `mdsearch embed --model B` | The global model switches to B and `Notes` is rebuilt under B |
| EX-005 | `Notes` has files but its lexical index was never built | I run `mdsearch embed` | `Notes` is skipped and reported in the summary |
| EX-006 | `Notes` has files but its lexical index was never built | I run `mdsearch embed --collection Notes` | The command fails and reports the index is not built |
| EX-007 | `Notes` exists but has no stored files | I run `mdsearch embed` | `Notes` is skipped and reported in the summary |
| EX-008 | No collection named `Journal` exists | I run `mdsearch embed --collection Journal` | The command fails and reports the collection was not found |
| EX-009 | The model is not cached locally | I run `mdsearch embed --model NAME` | The command fails with a clear error naming the model and suggesting `--download` |
| EX-010 | The model is not cached locally and the network allows it | I run `mdsearch embed --download` | The model assets are fetched and `embed` runs in the same invocation |
| EX-011 | `--download` is passed but the fetch fails | I run `mdsearch embed --download` | The command fails cleanly and no collection is modified |
| EX-012 | `--model BOGUS` is passed and BOGUS is unsupported | I run `mdsearch embed --model BOGUS` | The command fails with a clear error before any collection work |
| EX-013 | Collections `Notes` and `Archive` are to be embedded and `Archive` fails mid-build | I run `mdsearch embed` | `Notes` completes, `Archive` is reported failed, and the exit status reflects the failure |
| EX-014 | No database exists at the selected path | I run `mdsearch embed --database PATH` | The command fails and reports the database does not exist |

## Acceptance Criteria

- `mdsearch embed` builds a per-passage semantic index over the same passage
  units as the lexical index, keyed to the same passage rows.
- Without `--collection`, every collection is processed; `--collection NAME`
  restricts processing to one collection.
- Unbuilt lexical indexes are skipped in all-collections mode and fail when
  explicitly targeted; collections with no files are skipped silently whether
  targeted or not; unknown collections named by `--collection` fail.
- Per-collection rebuilds happen only when the stored file set or the global
  model changed; unchanged collections are reported as already current.
- Each collection's rebuild is atomic (all-or-nothing).
- A single global model is recorded; a different `--model` switches the model
  and rebuilds every collection with an existing semantic index.
- Unsupported models fail with a clear error before any collection work.
- A missing local model fails before any collection work unless `--download` is
  passed; `--download` fetches and embeds in one run, and a failed download
  modifies no collection.
- Per-collection failures are reported and processing continues; the exit status
  reflects any failure.
- The output is a per-collection summary plus the model used.
- A missing database fails without creating a file.

## Scope Boundaries

### In Scope

- The dedicated `mdsearch embed` command with `--collection`, `--model`,
  `--download`, and `--database` switches.
- Building and atomically rebuilding the per-passage semantic index keyed to the
  lexical passage rows.
- Rebuild-on-change logic based on stored file content hashes and the global
  model.
- Recording a single global embedding model and the per-collection embed state.
- Per-collection summary output and failure handling.
- Local model asset provisioning via `--download`.

### Out Of Scope

- Hybrid search and lexical-semantic result fusion (next EPIC-004 slice).
- Ranking, querying, or retrieving from the semantic index (next slice).
- JSON output, diff-style positions, and provenance (EPIC-006).
- Related-concept links and entity graphs (EPIC-005).
- Changing how the lexical index is built (US-006 behavior).
- Model selection and default (resolved in design and ADR).

## Dependencies

- `US-006` provides the built per-passage FTS5 index and the `passage_files`
  mapping whose passage rows the semantic index keys to.
- SQLite and the `sqlite-vector` extension provide vector storage and search
  (ADR-001, ADR-003).
- `fastembed` provides local embedding generation (ADR-001); the specific model
  is selected in design.
- ADR-004's evaluation framework guides model choice and later hybrid tuning.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Which local embedding model is the default, and what is the supported model set? | Yes, for design | TBD | Resolved in ADR-006: default `all-MiniLM-L6-v2`; supported set is fastembed's model set |
| OQ-002 | Where are model assets cached, and how is `--download` integrated with that cache? | No | TBD | Resolved in design: fastembed's local cache, gated by `--download` (ADR-006) |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable
