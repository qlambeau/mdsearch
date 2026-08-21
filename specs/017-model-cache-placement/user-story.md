---
id: US-017
title: "Model downloads live under .mdsearch with reliable availability detection"
type: user-story
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: PRD-001
epic: EPIC-011
feature: 017-model-cache-placement
related:
  - US-010
  - US-011
---

# User Story

## Story Card

As a developer-curator and coding-agent harness,
I want `mdsearch embed` and `mdsearch hybrid --download` to store model assets
under the `.mdsearch` data directory and to know reliably whether a model has
already been downloaded,
so that everything the tool persists lives in one place and I am never told to
pass `--download` for a model I have already downloaded.

## Context And Value

Today `--download` fetches the embedding model (and reranker) into a cache
location resolved per-process from `HF_HOME`, `FASTEMBED_CACHE_DIR`, or a
`.fastembed_cache` directory relative to the current working directory
(OBS-014). The embedded database, by contrast, always lives under
`~/.mdsearch/collections.db`. The model assets therefore land outside the
product's own data directory, in a location the user cannot predict.

The availability check (`model_is_cached`) is a strict probe of fastembed's
internal hf-hub layout (`refs/main` pointer plus a single primary file under a
commit-named snapshot folder) against that resolved location (OBS-015). Any
run whose working directory or environment differs from the one that performed
the download fails the probe, and `mdsearch embed` answers with
"embedding model X is not available locally; pass --download to fetch it" even
though the model was already downloaded — prompting repeated full
re-downloads.

This story makes the model cache a product-owned location under
`~/.mdsearch/models` (environment overrides still win), and replaces layout
probing with a completion marker written when a download finishes, so
availability is reported truthfully.

## Business Rules

- The default model cache directory is `~/.mdsearch/models`; the previous
  working-directory `.fastembed_cache` fallback is removed.
- The cache resolution order is: `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then
  `~/.mdsearch/models`. A user-set environment variable wins over the
  `.mdsearch` default.
- `--database` does not change the model location: models always default to
  `~/.mdsearch/models` regardless of where the database file lives.
- The rules apply to both the embedding model and the re-ranker model.
- A successful `--download` writes a completion marker in the resolved cache
  directory; "downloaded" means the marker for that model is present.
- Availability detection does not depend on the internal hf-hub cache layout
  (`refs/main` snapshots).
- The user-facing "not available locally; pass --download to fetch it" message
  stays as-is, but is now only shown when the model was genuinely not
  downloaded at the resolved location.
- Legacy downloads in a previous `.fastembed_cache` location are not migrated
  or reused; the user re-downloads once after upgrading.

## Examples

| Example | Given | When | Expected outcome |
| --- | --- | --- | --- |
| EX-001 | No env variables set | I run `mdsearch embed --download` from any working directory | Model assets land in `~/.mdsearch/models` |
| EX-002 | The model was downloaded via `--download` | I run `mdsearch embed` without `--download` from a different working directory | Embedding proceeds; no "pass --download" advice |
| EX-003 | `HF_HOME` (or `FASTEMBED_CACHE_DIR`) is set | I run `mdsearch embed --download` | Model assets land under the environment location, which is where availability is checked |
| EX-004 | `--download` is interrupted mid-fetch | I run `mdsearch embed` | "pass --download to fetch it" is shown; the marker was not written |
| EX-005 | A reranker is requested with `--download` | I run `mdsearch hybrid --reranker ... --download` | The reranker assets land in the same resolved cache; a later run without `--download` finds them |
| EX-006 | The user passes `--database /elsewhere/collections.db` | I run `mdsearch embed --download` | Model assets still land in `~/.mdsearch/models` |

## Acceptance Criteria

- `mdsearch embed --download` with no relevant environment variables stores the
  model in `~/.mdsearch/models`, regardless of the current working directory or
  a `--database` override.
- `mdsearch embed` succeeds without `--download` when the model's completion
  marker is present, regardless of the current working directory.
- `HF_HOME` or `FASTEMBED_CACHE_DIR` overrides the default cache location for
  both download and availability checks.
- An interrupted or partial download does not count as "downloaded": the
  availability check requires the completion marker.
- The re-ranker model follows the same location and marker rules as the
  embedding model.
- No regression in the existing `embed`/`hybrid` flows: cached-model runs,
  `--download` runs, unsupported-model errors, and the existing error message
  text keep working.

## Scope Boundaries

### In Scope

- A product-owned default cache directory `~/.mdsearch/models` for embedding
  and re-ranker model assets.
- A completion-marker based availability check replacing the hf-hub layout
  probe.
- Keeping `HF_HOME` / `FASTEMBED_CACHE_DIR` as overrides in the resolution
  order.
- Adapter-level and CLI-level regression tests for location, marker detection,
  and interrupted downloads.

### Out Of Scope

- Migrating or reusing legacy `.fastembed_cache` downloads.
- Changing the "pass --download" message wording.
- Changing the default embedding model or the reranker model.
- Progress reporting during downloads or embedding (OBS-016).
- Other TODO.md observations (OBS-004, OBS-005, ...).

## Dependencies

- `US-010` (EPIC-004) provides the embedding generation path whose cache
  resolution this story changes.
- `US-011` (EPIC-004) provides the hybrid path with reranker provisioning.
- The embedding and reranker adapters (`embed-fastembed`) own the shared cache
  resolution and marker logic.
- `DEC-016` (PRD-001 decision log) records the promotion of OBS-014/OBS-015.

## Open Questions

| ID | Question | Blocking? | Owner | Status |
| --- | --- | --- | --- | --- |
| OQ-001 | Does the `~/.mdsearch/models` directory need to be created on demand (yes, mirroring `collections.db` handling), and does creation failure surface as a clear error? | No | TBD | Resolved: created on demand in the adapter |
| OQ-002 | Is the completion marker a single per-model file (e.g. `*.completed`) in the model's cache folder, and does it need to survive hf-hub layout changes? | No | TBD | Resolved: per-model marker file, independent of hf-hub layout |

## INVEST Check

- [x] Independent
- [x] Negotiable
- [x] Valuable
- [x] Estimable
- [x] Small enough for roughly 1 to 3 days
- [x] Testable