---
id: REQ-017
title: "Model downloads live under .mdsearch with reliable availability detection requirements"
type: feature-requirements
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-017
related:
  - US-010
  - US-011
  - REQ-010
  - REQ-011
  - ADR-006
  - ADR-007
---

# Requirements

## Purpose And Actors

### Purpose

Make the model asset cache a product-owned, predictable location and make
"downloaded" detection reliable. `--download` stores embedding and re-ranker
model assets under the `.mdsearch` data directory by default (`~/.mdsearch/models`),
with `HF_HOME` and `FASTEMBED_CACHE_DIR` still honored as overrides, and
availability is determined by a completion marker written when a download
finishes — never by probing fastembed's internal hf-hub cache layout — so
`embed`/`hybrid` never instruct the user to pass `--download` for a model that
has already been downloaded. The feature completes EPIC-011 (OBS-014, OBS-015).

### Actors And External Systems

- Developer-curator invoking the `mdsearch` CLI.
- Coding-agent harness invoking the `mdsearch` CLI.
- The local model asset cache: the resolved cache directory holding embedding
  and re-ranker model assets.
- The local collection database addressed by the default path or an explicit
  `--database PATH` override.
- The network, used only when `--download` is passed.

## Preconditions

- The user invokes `mdsearch embed` or `mdsearch hybrid` with the respective
  command's existing switches (`--model`, `--reranker`, `--download`,
  `--database`, `--collection`).
- The database exists; the lexical index exists for any collection being
  embedded or searched.
- The existing model-selection, download-gating, and atomic-rebuild contracts
  from `REQ-010` and the re-ranker provisioning contract from `REQ-011` remain
  in force except where this contract changes the cache location and
  availability semantics.

## Inputs And Outputs

| Interaction | Inputs | Outputs | Validation |
| --- | --- | --- | --- |
| Embed with download | `embed [--collection NAME] [--model NAME] [--reranker NAME] --download [--database PATH]` | Per-collection summary as in `REQ-010`; model assets fetched to the resolved cache directory and a completion marker written | The resolved cache directory is usable; the fetch succeeds and the marker is written before embedding proceeds |
| Embed without download | `embed [--collection NAME] [--model NAME] [--reranker NAME] [--database PATH]` | Per-collection summary as in `REQ-010`, or the existing "not available locally; pass --download" error | A completion marker exists for the effective model at the resolved cache directory |
| Hybrid with reranker | `hybrid QUERY --reranker NAME [--no_rerank ...] [--download]` | Ranked results as in `REQ-011`, or the existing re-ranker "pass --download" error | The re-ranker marker exists at the resolved cache directory (or `--download` is passed) |
| Cache location override | `HF_HOME` or `FASTEMBED_CACHE_DIR` set in the environment | Downloads and availability checks use the environment location | The environment variable names a usable directory |

## Functional Requirements

| ID | Requirement | Priority | Traceability |
| --- | --- | --- | --- |
| FR-001 | The model cache resolution order shall be `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then the product default `~/.mdsearch/models`. The working-directory `.fastembed_cache` fallback is removed. | Must | US-017; The model cache resolution order is honored |
| FR-002 | With no override set, `--download` shall store the embedding model assets in `~/.mdsearch/models` regardless of the current working directory. | Must | US-017; A download stores the model under ~/.mdsearch/models with no environment overrides |
| FR-003 | A `--database` override shall not change the model cache location: model assets are stored under the resolved default regardless of where the database file lives. | Must | US-017; A --database override does not change the model location |
| FR-004 | A successful `--download` shall write a completion marker for the model in the resolved cache directory; a model counts as downloaded if and only if its marker is present. | Must | US-017; A download stores the model under ~/.mdsearch/models with no environment overrides; Partial assets without a completion marker are not considered downloaded |
| FR-005 | `embed` without `--download` shall proceed when the effective model's completion marker is present at the resolved cache directory, regardless of the current working directory, and shall not advise passing `--download`. | Must | US-017; A previously downloaded model is found from a different working directory |
| FR-006 | When the effective model has no completion marker, `embed` without `--download` shall fail before any collection work with the existing "model is not available locally; pass --download to fetch it" error. Partial or interrupted downloads leave no marker and therefore fail under this rule. | Must | US-017; An interrupted download does not count as downloaded; Partial assets without a completion marker are not considered downloaded |
| FR-007 | The re-ranker model shall follow the same cache location resolution and completion-marker rules as the embedding model, for both download and availability checks. | Must | US-017; The reranker follows the same cache location and marker rules |
| FR-008 | When `HF_HOME` or `FASTEMBED_CACHE_DIR` is set, both the download target and the availability check shall use that location. | Must | US-017; HF_HOME overrides the default model cache location; The model cache resolution order is honored |
| FR-009 | All other `embed` and `hybrid` contracts remain unchanged: `REQ-010` FR-009/FR-010 (uncached-model gating and clean `--download` failure semantics), `REQ-011` re-ranker gating, the unsupported-model errors, the "pass --download" message text, per-collection outcomes, and the offline-by-default rule. | Must | US-017 (scope boundaries; acceptance criteria) |
| FR-010 | Model assets previously downloaded into a legacy `.fastembed_cache` location shall not be migrated or reused; the user re-downloads once after upgrading. | Must | US-017 (business rules; scope boundaries) |

## Postconditions And Invariants

- After a successful `--download`, the model's assets and its completion marker
  coexist in the resolved cache directory; the marker is written only when the
  download completed.
- A model with a marker at the resolved cache directory is never reported as
  "not available locally".
- The resolution order is stable per run: downloads and availability checks in
  one invocation use the same resolved directory.
- The default location is independent of the working directory and of
  `--database`.
- The command operates offline unless `--download` is explicitly passed;
  availability checks never touch the network.

## Edge And Failure Behavior

| Condition | Expected behavior | User-visible result |
| --- | --- | --- |
| Model marker present but its files were deleted externally | Treated as downloaded (marker contract); a later embed surfaces a storage error when building the session | Clear storage error at session build; no spurious "pass --download" advice |
| `--download` fetch succeeds but the marker cannot be written | Fail cleanly; no collection modified | Existing download-failure error semantics (`REQ-010` FR-010) |
| The resolved cache directory cannot be created | Fail before any collection work | Clear error naming the location |
| Interrupted or partial download | No marker written | Existing "pass --download" error on the next run, truthfully |
| Unsupported model or re-ranker name | Fail before any collection work (unchanged) | Existing clear error naming the model |
| Both `HF_HOME` and `FASTEMBED_CACHE_DIR` set | `HF_HOME` wins | Download and availability check use the `HF_HOME` location |

## Quality Requirements

- Availability detection is deterministic: a marker presence check, independent
  of fastembed's internal hf-hub layout (`refs/main` snapshots) and of the
  current working directory.
- The download location is predictable and documented: environment overrides
  win, otherwise `~/.mdsearch/models`.
- No additional latency on the `embed`/`hybrid` paths: the availability check
  is a single marker lookup.
- Offline default is preserved: no network access unless `--download` is
  passed.
- The README and CLI documentation describe the resolution order and the marker
  contract.

## Dependencies And Deferred Decisions

- The embedding and re-ranker adapters (`embed-fastembed`) share the cache
  resolution and marker logic; `REQ-010`/`REQ-011` contracts stay in force
  except for the changed cache semantics (R-SDD-05).
- An ADR records the cache resolution order, the completion-marker contract,
  and the removal of the working-directory fallback (story OQ-001/OQ-002
  resolution).
- Marker file naming and placement within the resolved cache directory are
  adapter-internal and deferred to design.
- The default embedding model and re-ranker model are unchanged.

## Traceability

- Source story: `US-017` in `user-story.md`
- Executable scenarios: `scenarios.feature`
- Parent PRD: `PRD-001` in `specs/prds/PRD-001.md` (EPIC-011, DEC-016)