# AGENTS.md

## Repository Purpose

This repository is an SDD Workflow Kit for turning product intent into
implementation-ready feature specifications. The current product context is
the `mdsearch` Markdown Knowledge Search CLI described by `PRD-001`.

## Read First

> **Authority:** `specs/SDD_WORKFLOW.md` is the **single normative authority** for the SDD lifecycle, artifact responsibilities, state transitions, gates, and the spec-ready predicate. `specs/CONSTITUTION.md` is the single normative authority for Rust engineering rules. `AGENTS.md` (this file) and `docs/SDD_WORKFLOW_KIT.md` are **derivative / informational** only — if they conflict with `specs/SDD_WORKFLOW.md` on workflow matters, `specs/SDD_WORKFLOW.md` wins; if `specs/SDD_WORKFLOW.md` conflicts with the Constitution on engineering form, the Constitution wins and the conflict MUST be raised.

- Read `specs/SDD_WORKFLOW.md` for the complete specification workflow and gates — **single normative source** for lifecycle, gates, readiness, artifact boundaries, identifier/template rules, and editing rules.
- Read the complete `specs/CONSTITUTION.md` before the first Rust edit in a session; it is the normative engineering authority for Rust.
- Read `docs/SDD_WORKFLOW_KIT.md` only as a **non-normative, informational** kit manual — it MUST NOT be treated as a gate definition; if it conflicts with `specs/SDD_WORKFLOW.md`, the latter wins.
- Read `README.md` for the `mdsearch` tool manual and repository orientation.
- Read the applicable PRD under `specs/prds/` before refining or implementing a feature.
- Read the complete feature packet under `specs/NNN-feature-slug/` before generating code.
- Read related ADRs under `specs/adr/` before making consequential technical decisions.
- Treat files under `specs/templates/` as source templates, not active product or feature specifications.

## SDD Workflow — Summary

> **Derivative summary.** The paragraphs below are NOT the gate definitions. For lifecycle, artifact responsibilities, state transitions, the spec-ready predicate, identifier/template rules, editing rules, and precedence, the **normative source is `specs/SDD_WORKFLOW.md`**; on Rust engineering form `specs/CONSTITUTION.md` wins and any conflict MUST be raised.

> **Skill mandate (normative):** Every SDD workflow step defined in `specs/SDD_WORKFLOW.md` §Skill Mapping MUST be executed via its mapped skill (`.agents/skills/`). Direct creation or editing of artifacts bypassing the skill, or manual `status` transitions bypassing `promote-artifact` / `verify-feature`, is non-conforming. See `specs/SDD_WORKFLOW.md` §Normative Authority & Precedence and §Skill Mapping.

Work on one independently valuable vertical feature slice at a time. The lifecycle, gates, and readiness predicate are defined in `specs/SDD_WORKFLOW.md` (Shared Artifact Lifecycle, Artifact Boundaries/Responsibilities, Spec-Ready Checklist, Constitutional Engineering Loop). In short: select a slice from an approved PRD → refine `user-story.md` (US-NNN) via `refine-user-stories` → `scenarios.feature` (`# parent`/`# status: approved`) via `user-story-to-gherkin` → `requirements.md` (REQ-NNN) via `create-requirements` → `design.md` (DES-NNN) + ADRs via `create-design` → `tasks.md` (TASK-NNN) via `create-tasks` → validate packet readiness via `promote-artifact` (all specs `approved`) → test-first implementation via `implement-feature` (`RED → GREEN → REFACTOR → TRACE`, `R-SDD-02`) → verification via `verify-feature` with project quality & domain gates defined by `specs/CONSTITUTION.md` and observed output in `tasks.md` → release via `record-release` / triage via `triage-observation` → update specs first if behavior diverges.

Stop and ask for clarification when a blocking product, domain, dependency, or behavior question remains. Do not invent missing requirements.

Artifact responsibilities, identifier allocation (`PRD-NNN`, `ADR-NNN`, `US-NNN`, `REQ-NNN`, `DES-NNN`, `TASK-NNN`, `CHART-NNN`, `DB-NNN`, `TABLE-NNN`, `OBS-NNN`, `REL-NNN`), template rules, and editing/verification expectations: see `specs/SDD_WORKFLOW.md` §Artifact Boundaries, §Identifier Rules, §Template Rules, §Editing and Verification. Product-specific constraints live in the approved PRD (e.g. `specs/prds/PRD-001.md` for this project), not in this workflow summary.

## Rust Constitution — Summary

> **Derivative summary.** The paragraphs below are NOT the engineering authority. The normative source is `specs/CONSTITUTION.md`.

`specs/CONSTITUTION.md` governs how Rust code is written. Specifications govern what the code must do. If they conflict, preserve the constitution's engineering rules, preserve the specification's behavior, and raise the conflict rather than silently choosing.

- Read the constitution itself before editing Rust; do not rely on a summary.
- Do not add a crate-level dependency, workspace member, or architectural layer without explicit human approval in the current session.
- Write tests before implementation and do not weaken tests, delete assertions, add `#[ignore]`, or loosen lints to make a build pass.
- Do not write `unsafe` without the constitution's required human-authored ADR process.
- Do not amend `specs/CONSTITUTION.md` without explicit current-session human authorization and an accompanying ADR; `ADR-002` records the authorized repository-layout exception for this session.
- A Rust unit of work is incomplete until the constitution's required tooling and Definition of Done gates have been executed and observed.

The complete implementation packet includes `specs/CONSTITUTION.md`.

QMD and the personal wiki are optional research tools. They are not runtime dependencies and must not be required when the user has not requested research.
