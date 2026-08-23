# SDD Workflow Kit — Informational Manual

> **Non-normative — informational only.** This document is a **manual / orientation guide** for the spec-first workflow kit that scaffolds `mdsearch` development. It is **NOT normative**. The single normative authority for the lifecycle, artifact responsibilities, state transitions, gates, and the spec-ready predicate is **`specs/SDD_WORKFLOW.md`**. The single normative authority for Rust engineering rules is **`specs/CONSTITUTION.md`**. If any section below conflicts with either of those two files, **the normative file wins** — do not treat this manual as a gate definition.
>
> This manual was relocated from the repository `README.md`. The current `README.md` is the `mdsearch` end-user manual.

## Purpose

This workspace contains a reusable, spec-first workflow kit for starting projects
with product intent, refining just-in-time stories, and turning confirmed
behavior into executable Gherkin contracts. This manual helps newcomers orient;
the workflow itself is defined in `specs/SDD_WORKFLOW.md`.

The kit is portable. It does not require QMD or the personal wiki at runtime.
Use QMD only when a user explicitly wants background research and the target
environment provides it.

## Workflow Overview (Summary — See `specs/SDD_WORKFLOW.md` for Gates)

> The numbered list below is a **reading guide**, not the gate definition. The flowchart, lifecycle table, artifact responsibilities, spec-ready predicate, and skill mapping in `specs/SDD_WORKFLOW.md` are authoritative.

1. **Create a PRD** with `create-prd`.
   - Project and major-feature PRDs: `specs/prds/PRD-NNN.md`.
   - The interview asks one question at a time and writes only after approval.
2. **Refine one story** with `refine-user-stories`.
   - Select an epic from an approved PRD.
   - The default output is one story in `specs/NNN-feature-slug/user-story.md`.
   - The interview stops on blocking uncertainty and writes only after approval.
3. **Formulate executable behavior** with `user-story-to-gherkin`.
   - A canonical story produces `specs/NNN-feature-slug/scenarios.feature`.
   - Scenarios carry `# parent: US-NNN` and `# status: approved` headers.
   - Ambiguity must be resolved before scenarios are written.
4. **Complete specifications** with `create-requirements`, `create-design`, and `create-tasks`.
   - `requirements.md` defines observable behavior and contracts.
   - `design.md` defines the technical approach, schema updates, and ADRs.
   - `tasks.md` defines ordered implementation, red-test steps, and verification work.
5. **Promote specifications** with `promote-artifact`.
   - Explicitly validates preconditions and transitions `draft -> in-review -> approved`.
   - All packet specifications must be in `status: approved` before implementation starts (see the spec-ready predicate in `specs/SDD_WORKFLOW.md`).
6. **Implement and verify** with `implement-feature` and `verify-feature`.
   - Follows the constitutional red-to-green loop (`SPEC -> RED TEST -> IMPLEMENT -> GREEN TEST -> REFACTOR -> TRACE`) defined in `specs/CONSTITUTION.md` and the workflow diagram in `specs/SDD_WORKFLOW.md`.
   - Runs the project's required quality and domain evaluation gates defined by `specs/CONSTITUTION.md` (and any domain evaluation criteria) and logs observed output in `tasks.md`.
7. **Release and archive** with `record-release`.
   - Compiles a release milestone in `specs/releases/REL-NNN.md`.
   - Completed feature packets are moved to `specs/archive/` with `status: archived`.
8. **Triage observations** with `triage-observation`.
   - Intakes, classifies, and routes post-release defects or debt in `specs/TODO.md`.

For the normative flowchart, artifact lifecycle, responsibilities, spec-ready checklist, skill mapping, and constitutional loop, read `specs/SDD_WORKFLOW.md` directly.

## Project Layout

```text
specs/
|-- SDD_WORKFLOW.md            # ← normative workflow (lifecycle, gates, readiness) — START HERE
|-- CONSTITUTION.md            # ← normative Rust engineering rules — START HERE for Rust
|-- prds/
|   |-- PRD-001.md
|-- templates/
|   |-- project/
|   |-- feature/
|   |-- supporting/
|-- adr/
|-- charts/
|-- schema/
|-- releases/
|   |-- REL-001.md
|-- 001-feature-slug/
|   |-- user-story.md
|   |-- scenarios.feature
|   |-- requirements.md
|   |-- design.md
|   |-- tasks.md
`-- archive/
```

The normative Rust engineering constitution is `specs/CONSTITUTION.md` and must
be read before editing Rust. `specs/SDD_WORKFLOW.md` is the normative workflow.
Optional project context files can be added under `specs/`: `product.md`, `tech.md`,
`context.md`, and `glossary.md`. The canonical ADR template is `specs/templates/supporting/adr.md`.

## Where To Read First (Informational Guide)

This manual suggests an order; the authorities remain the two `specs/` files above:

1. `specs/SDD_WORKFLOW.md` — workflow, lifecycle, gates, readiness (normative).
2. `specs/CONSTITUTION.md` — Rust engineering rules (normative).
3. `specs/prds/PRD-001.md` — product intent for `mdsearch`.
4. `AGENTS.md` — operational read-first guide (derivative, points to the two normative files).
5. This file (`docs/SDD_WORKFLOW_KIT.md`) — kit manual (this file, informational).

## Adoption

The workflow kit embeds the following skills under `.agents/skills/` — each workflow step in `specs/SDD_WORKFLOW.md` §Skill Mapping MUST be executed via its mapped skill (normative, see `specs/SDD_WORKFLOW.md` §Normative Authority & Precedence and §Skill Mapping). Bypassing the skill is non-conforming.
- `create-prd`
- `refine-user-stories`
- `user-story-to-gherkin`
- `create-requirements`
- `create-design`
- `create-tasks`
- `promote-artifact`
- `implement-feature`
- `verify-feature`
- `triage-observation`
- `record-release`
- `qmd` (optional bootstrap)

Use `specs/templates/` only via the skills (templates are source material for the skills, not a bypass). Keep the kit's templates
available to the skills; do not assume this workspace is the target project.
Skill behavior is governed by `specs/SDD_WORKFLOW.md` and `specs/CONSTITUTION.md`; this manual does not override skill contracts.

## Statuses & Gates (Summary — Normative Definition Lives in `specs/SDD_WORKFLOW.md`)

All artifacts follow the shared state lifecycle:

- `draft`
- `in-review`
- `approved`
- `implemented`
- `archived`
- `superseded`

The exact meanings, promotion conditions, and the spec-ready predicate are defined in `specs/SDD_WORKFLOW.md` §Shared Artifact Lifecycle / §Spec-Ready Checklist. Do not rely on the list above as a gate definition.

No skill overwrites an existing artifact silently. Upstream artifacts must be in `status: approved` before downstream artifacts can be generated or promoted — per `specs/SDD_WORKFLOW.md`.
