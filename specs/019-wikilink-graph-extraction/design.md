---
id: DES-019
title: "Wikilink graph extraction design"
type: feature-design
status: approved
created: 2026-08-22
updated: 2026-08-22
owner: TBD
parent: US-019
related:
  - REQ-019
  - US-019
  - ADR-014
  - REQ-012
  - US-012
---

# Design

## Context And Constraints

EPIC-013 makes knowledge graph extraction recognize Obsidian-style wikilinks
(`REQ-019`): `[[target]]`, `[[target|label]]`, `[[path/target#heading]]`, and
`[[target#heading|label]]` become `LINKS_TO` edges; labels are ignored; header
fragments are stripped; resolution is case-insensitive with ambiguity skipping;
self-edges are skipped; standard markdown links keep their exact behavior.

Today only `inline_markdown_links` runs in `extract_graph`
(`crates/domain/src/graph.rs:348-357`), scanning for `](` and resolving via
`resolve_file` (`graph.rs:422-484`). Wikilinks are not recognized, so
wikilink-based vaults yield zero `LINKS_TO` edges (OBS-009).

The constitution governs the implementation: the domain stays pure and
deterministic (R-SEP-02, R-DIR-02); the change is additive with no schema,
store, adapter, or CLI impact; tests come first (R-TST-01). The approved
decision record is ADR-014.

## Proposed Design

Two new pure functions in `crates/domain/src/graph.rs` plus a small
integration in `extract_graph`:

1. **`inline_wikilinks(content: &[u8]) -> Vec<String>`** — scans the raw text
   for `[[`, closes at the first `]]`, and returns the path part of the inner
   text: everything before the first `|` (label dropped) and before the first
   `#` (fragment dropped). Inner text that is empty after stripping — bare
   `[[#heading]]`, `[[]]` — is dropped, as are `http://`/`https://` targets.

2. **`resolve_wikilink(target, source_path, known) -> Option<String>`** —
   mirrors `resolve_file`'s candidate strategy (target as-is; joined onto the
   source file's parent; `.md`-appended variants of both; basename match) but
   matches known paths with `eq_ignore_ascii_case`. A candidate with exactly
   one case-insensitive match resolves to it; a candidate with more than one
   match is ambiguous and skipped. `resolve_file` is untouched.

3. **`extract_graph` integration** — after the existing markdown-link loop, a
   wikilink loop: for each `inline_wikilinks` target, `resolve_wikilink`; on
   resolution, insert the `LINKS_TO` edge unless the resolved path equals the
   source file's own path (self-edge skip). Markdown-link behavior is
   unchanged.

No changes to `resolve_file`, frontmatter handling, `extract_graph`'s other
passes, or any other layer.

## Components And Responsibilities

| Component | Responsibility | Depends on |
| --- | --- | --- |
| `inline_wikilinks` (domain, new) | Extract wikilink path targets from raw content, label/fragment stripped, `[[#...]]`/`[[]]`/http dropped | — |
| `resolve_wikilink` (domain, new) | Case-insensitive resolution against known paths with ambiguity skipping | `Path`, known-path set |
| `extract_graph` (domain, existing) | Add the wikilink pass after the markdown-link pass; skip self-edges | `inline_wikilinks`, `resolve_wikilink` |

## Interfaces And Contracts

| Interface | Inputs | Outputs | Errors |
| --- | --- | --- | --- |
| `inline_wikilinks` (internal) | `&[u8]` content | `Vec<String>` path targets (empty when none) | — (total) |
| `resolve_wikilink` (internal) | `&str` target, `&Path` source path, `&HashSet<String>` known paths | `Option<String>` resolved path | — (total; ambiguous/unresolved → `None`) |
| `extract_graph` (domain) | `GraphFile` inputs (unchanged) | `EntityGraph` with wikilink `LINKS_TO` edges added | unchanged |

Both new functions are total and deterministic; the resolution contract
(`None` for unresolved or ambiguous) mirrors `resolve_file`'s `None`
semantics.

## Data And State Flow

```mermaid
flowchart TD
    EXTRACT["extract_graph(file, known)"]
    MD["markdown-link pass (unchanged): inline_markdown_links + resolve_file"]
    WI["wikilink pass: inline_wikilinks(content)"]
    RESOLVE["resolve_wikilink(target, file.path, known)"]
    SELF{"resolved == source path?"}
    EDGE["insert LINKS_TO edge source -> resolved"]
    SKIP["no edge (unresolved / ambiguous / self / no target)"]

    EXTRACT --> MD --> WI --> RESOLVE
    RESOLVE -->|Some(path)| SELF
    SELF -->|no| EDGE
    SELF -->|yes| SKIP
    RESOLVE -->|None| SKIP
```

The graph build stays deterministic: identical content yields identical edges,
and a rebuild regenerates them (fingerprint-driven, unchanged).

## Security, Performance, And Operations

- Security: no new input surface; content is parsed as bytes with no
  allocation amplification (one scan per `[[` occurrence).
- Performance: the wikilink pass is one linear scan plus one case-insensitive
  lookup per target; ambiguity checks compare against known paths once per
  candidate.
- Operations: no schema, migration, store, adapter, or CLI changes; existing
  databases rebuild edges on the next `update`.

## Alternatives Considered

| Alternative | Why not chosen |
| --- | --- |
| Alias nodes / `ALIAS_OF` edges for piped labels | Richer but heavier; the markdown-link form sets the label-ignoring precedent (ADR-014) |
| Case-sensitive wikilink resolution | Leaves common `[[Note]]`-to-`note.md` links dangling (ADR-014) |
| Make `resolve_file` case-insensitive for all forms | Changes existing markdown-link behavior (ADR-014) |
| Code-fence-aware wikilink extraction | OBS-010's scope; kept out of this slice |
| Embed (`![[note]]`) and block (`[[^block]]`) support | Different semantics; out of scope |

## Risks And Open Decisions

- Ambiguity: a vault with both `Note.md` and `note.md` makes `[[Note]]`
  unresolvable — a deliberate, documented tradeoff (ADR-014), covered by
  tests.
- Wikilinks in code fences can still produce phantom edges; deferred to
  OBS-010 and documented in the story's out-of-scope.
- Self-markdown-link behavior is preserved as-is, which is slightly
  inconsistent with wikilink self-skipping; accepted to keep the change
  additive (ADR-014).
- No open decisions remain; story OQ-001 (no schema change needed) is resolved
  by this design.

## Verification Approach

- Domain unit tests (`crates/domain/src/graph.rs` test module): every
  wikilink form; label and fragment stripping; `[[#heading]]` and `[[]]` and
  `http` targets produce nothing; case-insensitive resolution; ambiguity
  skipping; unresolved targets; self-edge skipping; markdown links still
  extracted alongside wikilinks; determinism (repeated extraction identical).
- Store integration tests (`012-entity-graph` packet): graph rebuild from
  files containing wikilinks produces the expected `LINKS_TO` edges; deleted
  targets drop edges on rebuild.
- CLI: `update`/`graph neighbors` acceptance behavior unchanged in shape;
  existing `crates/app/tests` graph tests pass.
- Run the constitution gates from `specs/CONSTITUTION.md` (R-TOOL-04) and
  observe red-then-green on the new tests (R-TST-01) before marking the
  implementation complete.