# AGENTS.md

## Repository Purpose

This repository is an SDD Workflow Kit for turning product intent into
implementation-ready feature specifications. The current product context is
the `kv` Markdown Knowledge Search CLI described by `PRD-001`.

## Read First

- Read `README.md` for repository orientation and layout.
- Read `SDD_WORKFLOW.md` for the complete specification workflow and gates.
- Read the complete `specs/CONSTITUTION.md` before the first Rust edit in a session; it is the normative engineering authority.
- Read the applicable PRD under `specs/prds/` before refining or implementing a feature.
- Read the complete feature packet under `specs/NNN-feature-slug/` before generating code.
- Read related ADRs under `specs/adr/` before making consequential technical decisions.
- Treat files under `specs/templates/` as source templates, not active product or feature specifications.

## Required SDD Workflow

Work on one independently valuable vertical feature slice at a time.

1. Select a slice from an approved PRD.
2. Refine an approved `user-story.md` with actor, goal, value, rules, examples, scope, dependencies, and testable acceptance criteria.
3. Create one sibling `scenarios.feature` with unambiguous behavioral scenarios.
4. Complete `requirements.md` with externally observable inputs, outputs, validation, errors, invariants, and quality requirements.
5. Complete `design.md` with the technical approach, interfaces, state flow, risks, alternatives, and verification approach.
6. Record consequential or durable decisions in an ADR.
7. Complete `tasks.md` with ordered implementation work and concrete verification checks.
8. Generate or implement code only from the approved implementation packet.
9. Run tests and review the implementation against the approved specifications.
10. Update and approve the specifications before changing behavior discovered during implementation.

Stop and ask for clarification when a blocking product, domain, dependency, or
behavior question remains. Do not invent missing requirements.

## Rust Constitution

`specs/CONSTITUTION.md` governs how Rust code is written. Specifications govern
what the code must do. If they conflict, preserve the constitution's engineering
rules, preserve the specification's behavior, and raise the conflict rather
than silently choosing.

- Read the constitution itself before editing Rust; do not rely on a summary.
- Do not add a crate-level dependency, workspace member, or architectural layer without explicit human approval in the current session.
- Write tests before implementation and do not weaken tests, delete assertions, add `#[ignore]`, or loosen lints to make a build pass.
- Do not write `unsafe` without the constitution's required human-authored ADR process.
- Do not amend `specs/CONSTITUTION.md` as an agent.
- A Rust unit of work is incomplete until the constitution's required tooling and Definition of Done gates have been executed and observed.

The constitution's canonical Rust layout example uses root `CONSTITUTION.md`,
`docs/specs/`, and `docs/adr/`, while this workflow kit currently uses
`specs/CONSTITUTION.md`, `specs/`, and `specs/adr/`. Treat this as an unresolved
repository-layout conflict. Do not restructure or amend either authority
silently; resolve it with an explicit human decision before Rust implementation.

The complete implementation packet includes `specs/CONSTITUTION.md`.

## Artifact Boundaries

- PRDs define product intent, outcomes, scope, constraints, and boundaries.
- User stories define business intent and observable value without prescribing implementation.
- Gherkin scenarios define executable behavioral examples.
- Requirements define externally observable contracts.
- Design documents define how approved behavior will be realized.
- ADRs preserve consequential decisions, alternatives, and tradeoffs.
- Tasks define implementation order and verification, but must not silently change behavior.

## Identifier Rules

Use independent monotonically increasing sequences for PRD, ADR, and user-story
artifact types.

- PRDs use `PRD-NNN` and live at `specs/prds/PRD-NNN.md`.
- ADRs use `ADR-NNN` and live under `specs/adr/`.
- User stories use `US-NNN` and live at `specs/NNN-feature-slug/user-story.md`.
- `NNN` is exactly three zero-padded decimal digits.
- Each prefix has its own sequence; creating `ADR-001` does not advance PRD or US numbering.
- Allocate the next number above every existing active, archived, or superseded artifact of the same type.
- Never reuse an ID, including after deletion, archiving, or supersession.
- Preserve an ID when revising or moving an artifact.
- Template placeholders such as `PRD-NNN`, `ADR-NNN`, and `US-NNN` do not consume numbers.
- Scan existing artifacts before allocating an ID and stop if a collision is found.

The current approved project PRD is `specs/prds/PRD-001.md`. The first concrete
ADR and user story may use `ADR-001` and `US-001`; their former illustrative
template values do not reserve those numbers.

## Template Rules

- Use templates from `specs/templates/` when creating new artifacts.
- Replace all illustrative content, placeholder metadata, and example domain behavior before approval.
- Never treat the former `specs/001-short-feature-slug/` directory or its sample delivery-address content as a real feature.
- Never treat `specs/templates/supporting/adr.md` as an approved ADR.
- Do not create downstream feature files merely as empty placeholders; create them when the preceding artifact is ready.

## Product Constraints

Preserve these approved `kv` constraints unless the PRD or an ADR explicitly
changes them:

- Local-first and offline operation are the defaults.
- External services are opt-in through CLI switches.
- Delivery is a single compiled Rust binary.
- All collections live in one embedded database file per machine.
- Indexing is driven by explicit update commands; there is no file watching.
- The tool retrieves grounded content but does not generate answers.
- Human-readable output is the default; richer machine-readable JSON is opt-in.
- Web UI, server mode, multi-user access, authentication, cloud sync, and hosted collections are out of scope.

Resolve database-engine and model decisions just in time for the feature that
needs them. Do not silently turn an open question into an architectural fact.

## Editing And Verification

- Inspect existing files and related specifications before editing.
- Do not overwrite an existing artifact silently.
- Keep front matter, IDs, parent references, statuses, and traceability consistent.
- Verify changed Markdown paths and internal references.
- Keep Mermaid diagrams in fenced `mermaid` blocks.
- For documentation-only changes, verify paths, links, IDs, and stale references.
- Run relevant implementation checks when application code exists.
- For Rust code, execute the constitution's required gates and report observed command output; do not claim completion from intent.
- Do not commit or publish changes unless explicitly requested.

QMD and the personal wiki are optional research tools. They are not runtime
dependencies and must not be required when the user has not requested research.
