# SDD Workflow Kit

This workspace contains a reusable, spec-first workflow for starting projects
with product intent, refining just-in-time stories, and turning confirmed
behavior into executable Gherkin contracts.

The kit is portable. It does not require QMD or the personal wiki at runtime.
Use QMD only when a user explicitly wants background research and the target
environment provides it.

## Workflow

1. **Create a PRD** with `create-prd`.
   - Project and major-feature PRDs: `specs/prds/PRD-NNN.md`.
   - The interview asks one question at a time and writes only after approval.
2. **Refine one story** with `refine-user-stories`.
   - Select an epic from an approved PRD.
   - The default output is one story in `specs/NNN-feature-slug/user-story.md`.
   - The interview stops on blocking uncertainty and writes only after approval.
3. **Formulate executable behavior** with `user-story-to-gherkin`.
   - A canonical story produces `specs/NNN-feature-slug/scenarios.feature`.
   - Ambiguity must be resolved before scenarios are written.
4. **Complete task specifications** using the feature templates.
   - `requirements.md` defines observable behavior and contracts.
   - `design.md` defines the technical approach and risks.
   - `tasks.md` defines ordered implementation and verification work.
5. **Implement and verify** against the approved artifacts and executable
   scenarios.
6. **Archive completed feature specs** under `specs/archive/` with
   `status: archived` when they are no longer active implementation context.

## Project Layout

```text
specs/
|-- prds/
|   |-- PRD-001.md
|-- templates/
|   |-- project/
|   |-- feature/
|   |-- supporting/
|-- adr/
|-- 001-feature-slug/
|   |-- user-story.md
|   |-- scenarios.feature
|   |-- requirements.md
|   |-- design.md
|   |-- tasks.md
`-- archive/
```

The normative Rust engineering constitution is `specs/CONSTITUTION.md` and must
be read before editing Rust. Optional project context files can be added under
`specs/`: `product.md`, `tech.md`, `context.md`, and `glossary.md`. The canonical
ADR template is
`specs/templates/supporting/adr.md`.
The v1 skills do not create or require these files.

## Adoption

The scaffold script is self-contained: it embeds the workflow skills and writes
them to `.agents/skills/` inside the target project (create-prd,
refine-user-stories, user-story-to-gherkin, and the optional qmd bootstrap).
Use `specs/templates/` when creating new artifacts. Keep the kit's templates
available to the skills; do not assume this workspace is the target project.

The skills use the current working directory by default and accept an explicit
target project path when provided.

## Template Rules

- Templates use placeholders such as `PRD-NNN`, `ADR-NNN`, and `US-NNN`.
- PRD, ADR, and user-story IDs each use an independent monotonically increasing
  sequence and are never reused.
- Front matter includes status, dates, ownership, and related artifacts.
- `TBD`, open questions, and blocking questions are explicit rather than
  silently guessed.
- Worked examples are generic and marked as illustrative. Replace them before
  using a template.
- Product requirements express why and what. Technical design expresses how.
- User stories contain readable criteria and examples. Gherkin is the executable
  confirmation kept in a separate file.

## Statuses

Use the shared lifecycle where it fits:

- `draft`
- `in-review`
- `approved`
- `implemented`
- `archived`
- `superseded`

`create-prd` and `refine-user-stories` set `approved` only after explicit user
confirmation. No skill overwrites an existing artifact silently.
