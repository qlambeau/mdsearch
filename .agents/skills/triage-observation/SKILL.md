---
name: triage-observation
description: >-
  Record, classify, and triage engineering observations, bugs, or technical debt
  in the TODO catalogue (specs/TODO.md) or dedicated observation records.
---

# Triage Observation

## Purpose

This skill standardizes the intake, investigation, and routing of post-release defects,
architectural friction, and performance bottlenecks discovered during codebase reviews
or operational use.

## Invocation

Examples:

- `Record observation for SQLite connection WAL mode configuration`
- `Triage observation OBS-011 and assess priority for next release`

## Observation Classification

Every observation (`OBS-NNN`) must be classified into one of five standard kinds:
1. **Behavior inconsistency:** Two surfaces behave differently for the same conceptual input.
2. **Architecture constraint:** A constant or schema choice narrows an advertised capability.
3. **Robustness / correctness:** Fragile detection or silent fallback that could mask failures.
4. **Scalability / performance:** Algorithmic behavior that grows poorly with collection scale.
5. **Maintainability / DRY:** Duplicated or scattered implementation.

## Triage Actions & Routing

1. **Intake & Verification:** Confirm observed behavior against exact codebase line numbers.
2. **Impact Assessment:** Detail user/caller impact, severity, and open decisions.
3. **Epic Promotion:** If selected for an upcoming release, follow `specs/prd_lifecycle_and_evolution_plan.md`:
   - Update `PRD-001` with a new `EPIC-NNN` and `DEC-NNN`.
   - Mark `OBS-NNN` status as `PROMOTED` in `specs/TODO.md`.
   - Refine into a vertical feature slice with `refine-user-stories`.
