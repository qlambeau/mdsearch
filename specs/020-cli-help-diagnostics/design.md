---
id: DES-020
title: "CLI help, diagnostics, and global options design"
type: feature-design
status: approved
created: 2026-09-30
updated: 2026-09-30
owner: Quentin
approval: "Quentin explicitly approved DES-020 as written in chat on 2026-09-30"
parent: US-020
depends_on: []
requires: []
blockers: []
related:
  - REQ-020
  - ADR-001
  - ADR-012
---

# Design

## Context And Constraints

US-020, its scenarios, and REQ-020 are approved. This slice changes invocation,
help, and diagnostics within the current command grammar. Existing collection,
indexing, retrieval, model-download, and output contracts remain the execution
contracts. PRD-001 defines the actors and local-first product boundary.

The implementation uses the existing clap, std, thiserror, rstest, and tempfile
dependencies. ADR-001 preserves the existing database foundation; ADR-012
preserves model-cache precedence. No new dependency, workspace member, layer,
schema, or port is needed. Existing process configuration stays in app under
R-SEP-10. Tests precede implementation under R-TST-01 and cite REQ-020.

## Proposed Design

Parse arguments before resolving any default paths. Declare `--database` once
on the root `Cli` with clap's global option propagation, removing repeated leaf
definitions. Dispatch passes the selected database to every operational command.

Provide two library entry points sharing parsing and dispatch: preserve
`run(args, home_directory)` for existing injected-home callers, and add
`run_from_environment(args)` for the binary. The latter obtains HOME only when
path resolution requires a default. Both return `Result<String, AppError>`;
clap informational outcomes remain typed `AppError::Arguments` values and are
handled at the process boundary.

Resolve the database to a concrete PathBuf before opening adapters. Resolve
model-cache configuration only for embed/hybrid, preserving precedence
`HF_HOME`, `FASTEMBED_CACHE_DIR`, then `HOME/.mdsearch/models`. Read each relevant
environment setting once per invocation; read HOME at most once and only if
the database or model cache needs its default. Pass resolved cache paths to
the existing adapters. Do not manufacture a substitute home directory.

The binary uses clap's error renderer and exit classification for
`AppError::Arguments`: help/version write to stdout and exit 0; rejected
arguments write to stderr and exit 2. Other AppError variants retain the
operational stderr/exit-1 path. Add a typed `HomeUnavailable` variant with the
existing lowercase home-directory diagnostic. Stream-write failures exit 1.
Existing successful command output writing is preserved in this slice.

Add short collection and limit spellings to the existing switch declarations.
Rename the ingestion CLI field/spelling to `skip_unreadable`, passing its value
to the existing use cases without changing their behavior. Reject the old
`--force` spelling. Historical specifications describe that behavior under
the old spelling; REQ-020 FR-009 explicitly replaces the CLI spelling.

Add purpose/effect descriptions, applicable prerequisites/defaults, argument
help, and invocation examples to root, groups, and leaf commands. Correct the
README's global options, exit codes, short switches, and ingestion descriptions
to match this slice's current grammar.

## Components And Responsibilities

| Component | Responsibility | Depends on |
| --- | --- | --- |
| `cli.rs` | Current grammar, global database option, aliases, descriptions, examples, parser validation | clap |
| `invocation.rs` | Library entry points, parse-first flow, configuration resolution, command dispatch | CLI, model-cache resolution, command execution |
| `run.rs` | Construct existing adapters/use cases using concrete resolved paths | Existing application/domain/adapter APIs |
| `rendering.rs` | Existing human/JSON formatting moved without contract changes | Existing retrieval result types |
| `model_cache.rs` | Pure cache-precedence resolution from injected optional path values | std paths |
| `error.rs` | Existing typed errors plus missing-HOME error | thiserror, existing error types |
| `bin/mdsearch.rs` | Call environment entry point, write output/diagnostics, return exit code | Library entry point and clap error renderer |
| `lib.rs` | Declare modules and re-export the two entry points | Existing app modules |

`run.rs` currently has over 700 non-test lines. Moving invocation and rendering
to responsibility-specific app modules satisfies R-DIR-06 without introducing
an architectural layer. Split dispatch by command group as necessary to keep
function bodies within R-SEP-11. Preserve existing rendering assertions when
moving their tests.

## Interfaces And Contracts

| Interface | Inputs | Outputs | Errors |
| --- | --- | --- | --- |
| `run` | Arguments and injected home directory | Existing command output string | Existing typed AppError, including clap information |
| `run_from_environment` | Arguments | Same command output string | AppError including HomeUnavailable when a default requires HOME |
| Private path resolution | Selected database, relevant cache overrides, lazy home lookup | Concrete selected database/cache paths | HomeUnavailable |
| Process boundary | Library result | stdout success/information or stderr diagnostic; exit 0/1/2 | Stream-write failure returns exit 1 |

All new public entry points receive documentation, error contracts, and compiled
examples under R-DOC-01 through R-DOC-04. Configuration helpers and dispatch
remain private or crate-visible; library roots contain declarations/re-exports.

## Data And State Flow

1. Parse invocation. On help/version or rejected arguments, return the clap
   outcome immediately; no HOME lookup, database access, or download occurs.
2. For an operational command, resolve its global database and any required
   model-cache path. Missing HOME fails before adapter construction if a default
   needs it. Explicit database/cache overrides bypass the corresponding default.
3. Dispatch to the existing use case with resolved configuration.
4. Return its success output or typed error. The binary writes to the prescribed
   stream and returns the prescribed exit code.

Configuration has no persistent state. There is no database migration. Existing
use-case transactions own operational rollback; this slice adds no transaction
or download path. Users recover from missing defaults by supplying paths or
restoring HOME, and from argument errors by correcting the invocation.

## Security, Performance, And Operations

- Security: resolve paths without expanding shell expressions; do not mutate
  process-global environment in tests or log environment contents.
- Performance: parse and resolve once; information requests avoid adapters and
  model initialization.
- Operations: keep binaries thin under R-DIR-04. Model downloads still require
  the existing explicit opt-in. Exit-code and switch-spelling changes are
  intentional compatibility changes from the approved story.

## Alternatives Considered

| Alternative | Why not chosen |
| --- | --- |
| Pre-scan raw arguments for help and database flags | Duplicates clap grammar and mishandles nested commands and option values |
| Keep duplicate leaf database options and add root copies | Creates inconsistent selection and help across command depths |
| Substitute an empty home directory when HOME is missing | Risks selecting unintended relative default paths |
| Change every existing injected-home caller to an environment API | Unnecessary API/test churn; preserve deterministic injected-home entry point |
| Preserve `--force` as an alias | Conflicts with the approved replacement and old-switch rejection scenario |

## Risks And Open Decisions

- Global option propagation must be exercised at every depth so dispatch uses
  the supplied database consistently.
- Cache overrides must work without HOME when the database is also explicit.
- Parser diagnostics and semantic use-case errors must retain their distinct
  typed classifications; do not classify errors by matching message text.
- Rendering moves must preserve existing JSON fields and success formatting.
- No blocking decisions remain. These are bounded implementation choices within
  existing layers and approved behavior; no new consequential architecture
  decision or ADR is proposed.

## Verification Approach

- RED first: rstest subprocess cases for information without HOME, invalid
  arguments, operational failure, global option positions, defaults and
  boundaries, short switches, help descriptions/examples, and renamed ingestion.
- Use temporary databases/files and subprocess-specific environment overrides.
  Add missing-path ingestion cases rather than relying on chmod behavior.
- Pure configuration tests prove cache precedence and that home lookup is
  unnecessary when explicit paths suffice. No filesystem or environment mutation
  in unit tests; test real environment behavior through subprocesses.
- Preserve and run the existing CLI/retrieval tests, updating ingestion invocation
  spellings with the same assertions and REQ-020 traceability.
- Check every scenario against test evidence and inspect help/README examples.
- Through verify-feature run `cargo xtask validate-specs`, `cargo xtask ci`,
  and `cargo xtask eval`; record observed output in tasks.md after task approval
  and implementation.
