---
name: promote-artifact
description: >-
  Validate prerequisite checklists and advance the lifecycle status of a specification
  or feature packet (e.g. draft -> in-review -> approved -> implemented -> archived).
---

# Promote Artifact

## Purpose

This skill is the explicit state-control mechanism for the SDD workflow. It prevents
"ghost drafts" and unverified status claims by evaluating formal prerequisite checklists
before transitioning an artifact or packet's `status` frontmatter.

## Invocation

Examples:

- `Promote specs/014-unify-query-semantics/requirements.md to approved`
- `Promote specs/015-embedding-dimensions/ to spec-ready`
- `Promote specs/001-create-collection/ to archived`

## State Transitions

Allowed artifact states:
- `draft -> in-review`: When initial authoring is complete.
- `in-review -> approved`: When human reviewer explicitly confirms the specification.
- `approved -> implemented`: When `verify-feature` has verified passing red/green tests and quality gates.
- `implemented -> archived`: When the feature is part of a closed release milestone and moved to `specs/archive/`.
- `* -> superseded`: When replaced by a successor artifact (`supersedes: ID`).

## Checklist Validation Rules

Before advancing an artifact to `approved`:
1. **Parent Approval**: Ensure parent PRD/story is `status: approved`.
2. **Completeness**: No unresolved `TBD` or placeholder values in normative sections.
3. **Traceability**: Requirements trace to story acceptance criteria and named scenarios.
4. **Blockers**: No open blockers (`blockers: []` or all marked `resolved`).
5. **Bidirectional Schema Links**: Database and table references match.

Before advancing a packet to `spec-ready` (ready for implementation):
- `user-story.md`: `status: approved`
- `scenarios.feature`: `# status: approved`
- `requirements.md`: `status: approved`
- `design.md`: `status: approved`
- `tasks.md`: `status: approved`
- All referenced ADRs and schemas: `status: approved`
- All `depends_on` dependencies: `status: implemented`

## Review And Write

1. Read the target artifact and its related dependencies.
2. Check all items against the transition checklist.
3. Report any validation failures or missing upstream approvals.
4. If validation passes, update `status: <target_state>` and set `updated: YYYY-MM-DD` and `approval` metadata.
