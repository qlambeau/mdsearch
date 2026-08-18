---
id: REQ-010
title: "Build the semantic index with the embed command requirements"
type: feature-requirements
status: implemented
created: 2026-08-18
updated: 2026-08-18
owner: TBD
parent: US-010
related:
  - US-006
  - US-007
  - DES-006
  - DES-007
  - DES-010
  - ADR-001
  - ADR-003
  - ADR-004
  - ADR-006
  - DB-001
  - TABLE-004
  - TABLE-005
  - TABLE-007
  - TABLE-008
  - TABLE-009
---

# Requirements

## Purpose And Actors

### Purpose

Provide a dedicated `mdsearch embed` command that builds and maintains a
per-passage semantic (vector) index over the passages already indexed by the
lexical index, so that conceptual, non-keyword queries can be answered by the
next EPIC-004 slice (hybrid search). The command embeds offline by default,
downloads model assets only when explicitly requested, records a single global
embedding model, and reports per-collection status.

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- The local embedding-model asset cache.
- The network, used only when `--download` is passed.

## Preconditions

- The user invokes `mdsearch embed`, with optional `--collection NAME`,
  `--model NAME`, `--download`, and `--database PATH`.
- The database path is `~/.mdsearch/collections.db` unless `--database PATH` is
  supplied.
- The database exists and contains the collections to be embedded.
- The lexical index exists for any collection the command attempts to embed.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Embed all collections | Optional `--model NAME`, optional `--download`, optional `--database PATH` | Per-collection summary lines (embedded passage count, already current, skipped, or failed) plus the model used | The selected model is supported and cached (or `--download` is passed) before any collection work |
| Embed one collection | `--collection NAME`, optional `--model NAME`, optional `--download`, optional `--database PATH` | Same per-collection summary restricted to the named collection | `NAME` matched case-insensitively; the collection exists; its lexical index is built |
| Download model assets | `--download` | Model assets fetched to the local cache, then embed proceeds | The fetch succeeds; on failure nothing is modified |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | `mdsearch embed` shall build a per-passage semantic index over the same passage rows the lexical index covers. | Must | US-010; Embed builds the semantic index from the lexical passages |
| FR-002 | The embedded passage set shall be the body paragraphs and the four frontmatter fields (`title`, `tags`, `aliases`, `summary`) of each stored file, keyed to the same passage rows used by the lexical index. | Must | US-010; Embed builds the semantic index from the lexical passages |
| FR-003 | Without `--collection`, `embed` shall process every collection; with `--collection NAME`, it shall process only that collection, matched case-insensitively. | Must | US-010; Embed builds the semantic index from the lexical passages |
| FR-004 | A collection's semantic index shall be rebuilt only when its stored file set (content hashes) or the global embedding model changed since the last successful embed; otherwise the command shall report the collection as already current and not rebuild it. | Must | US-010; Re-running embed with unchanged files reports already current; Embed rebuilds after the file set changed |
| FR-005 | Each collection's rebuild shall be atomic: the old vectors are replaced by the new ones in one all-or-nothing transaction, and a failed rebuild shall leave the previous semantic index intact. | Must | US-010; Embed failure leaves the previous semantic index intact |
| FR-006 | The command shall record a single global embedding model in the database; the first successful `embed` run records the model used (from `--model` or the default). | Must | US-010; A --model switch rebuilds every embedded collection under the new model |
| FR-007 | If a later `embed` passes `--model` differing from the recorded global model, the command shall switch the global model and rebuild every collection that has an existing semantic index under the new model, regardless of the `--collection` scope of the invocation. | Must | US-010; A --model switch rebuilds every embedded collection under the new model; A --model switch rebuilds embedded collections even under a narrow scope |
| FR-008 | An unsupported `--model` value shall fail with a clear error before any collection work. | Must | US-010; Unsupported model fails before any collection work |
| FR-009 | Without `--download`, if the selected model's assets are not cached locally, the command shall fail before touching any collection, naming the model and suggesting `--download`. | Must | US-010; Missing local model fails before any collection work without --download |
| FR-010 | With `--download`, the command shall fetch the model assets and embed in the same run; if the fetch fails, the command shall fail cleanly and modify no collection. | Must | US-010; --download fetches the model and embeds in the same run; A failed --download modifies no collection |
| FR-011 | A collection whose lexical index is not built shall be skipped silently and reported in the summary when processed in all-collections mode. | Must | US-010; Unbuilt lexical index is skipped in all-collections mode |
| FR-012 | When `--collection` targets a collection whose lexical index is not built, the command shall fail and report that the index is not built. | Must | US-010; Unbuilt lexical index fails when explicitly targeted |
| FR-013 | A collection with no stored files shall be skipped silently and reported in the summary, whether targeted via `--collection` or processed in all-collections mode. | Must | US-010; A collection with no stored files is skipped and reported; A collection with no stored files is skipped even when targeted |
| FR-014 | When `--collection` names a collection that does not exist, the command shall fail and report that the collection was not found. | Must | US-010; Unknown collection fails when explicitly targeted |
| FR-015 | In all-collections mode, a failure to embed one collection shall be reported, processing shall continue with the remaining collections, and the command's exit status shall reflect that a failure occurred. | Must | US-010; A per-collection failure is reported and processing continues |
| FR-016 | The command's output shall be a per-collection summary reporting, for each collection: embedded passage count, already current, skipped (no files or unbuilt lexical index), or failed, plus the model used. | Must | US-010; Embed builds the semantic index from the lexical passages |
| FR-017 | `mdsearch embed` against a missing database shall fail semantically without creating a database file. | Must | US-010; Report a missing database without creating a file |
| FR-018 | `--database PATH` shall select the database used by `mdsearch embed`. | Must | US-010; Report a missing database without creating a file |

## Postconditions And Invariants

- After a successful embed, every processed collection has a semantic index
  whose vector set corresponds exactly to its lexical passage set at that point.
- A collection's embed state records the stored file content-hash set and the
  global model under which the vectors were built.
- There is exactly one recorded global embedding model in the database; a model
  switch rebuilds every previously embedded collection so no collection retains
  vectors from a superseded model.
- The command operates offline unless `--download` is explicitly passed.
- The operation does not modify the lexical index, the stored files, or the
  collections.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Unsupported model name | Fail before any collection work | Clear error naming the model |
| Model assets not cached locally, no `--download` | Fail before any collection work | Clear error naming the model and suggesting `--download` |
| `--download` fetch failure | Fail cleanly; no collection modified | Clear fetch-failure error; prior semantic indexes untouched |
| `--collection` targets an unknown collection | Fail | Error that the collection was not found |
| `--collection` targets a collection with an unbuilt lexical index | Fail | Error that the index is not built |
| Unbuilt lexical index in all-collections mode | Skip and report | Collection shown as skipped in the summary |
| Collection with no stored files | Skip and report | Collection shown as skipped in the summary |
| A storage error while rebuilding one collection | Report the failure and continue with the remaining collections | Summary shows the collection as failed; exit status reflects the failure |
| A storage error while rebuilding one collection in isolation | The rebuild is rolled back | The previous semantic index is unchanged |
| Files changed since the last embed | Rebuild the affected collection | Current passage set embedded |
| No files and no model changed | Do not rebuild | Collection reported as already current |
| A `--model` value differs from the recorded global model | Switch the global model and rebuild every embedded collection | All previously embedded collections show the new model |
| The database does not exist | Fail without creating a file | Output communicates the database does not exist |

## Quality Requirements

- The operation shall work offline by default and shall not require a network
  service; network use is limited to the explicit `--download` path.
- Embedding is driven by the explicit `embed` command; there is no file watching
  or automatic background indexing.
- The semantic index shall be suitable for future vector similarity search and
  lexical-semantic fusion (the hybrid search command itself is the next EPIC-004
  slice).
- The default embedding model and the supported model set are resolved in design
  and recorded in an ADR; this contract does not fix them.

## Traceability

- Source story: `US-010` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md`
