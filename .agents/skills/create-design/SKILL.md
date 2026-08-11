---
name: create-design
description: >-
  Create a technical feature design from an approved story, requirements
  contract, and Gherkin scenarios, recording consequential decisions as ADRs.
---

# Create Design

## Purpose

This skill explains how an approved feature will be realized. It converts
observable requirements into components, interfaces, state flow, data choices,
risks, alternatives, and verification. It must not silently change behavior.

## Invocation

Examples:

- `Create the design for specs/001-create-collection/`.
- `Design US-001 from its approved requirements and scenarios.`
- `Complete design.md and record any consequential decisions for this feature.`

Use the current working directory as the target project unless the user gives an
explicit project path. Resolve all paths relative to that root.

## Prerequisites

1. Resolve the target feature directory under `specs/NNN-feature-slug/`.
2. Read the complete approved `user-story.md`, `scenarios.feature`, and `requirements.md`.
3. Read the parent PRD, related ADRs, project context, and the complete `specs/CONSTITUTION.md`.
4. Inspect the current codebase when implementation code exists.
5. Inspect whether `design.md` already exists.

Stop without writing when requirements are not approved, behavior is ambiguous,
or a technical decision is required but no decision authority is available. Ask
the user rather than choosing a product behavior or silently adding an
architectural constraint.

## Design Rules

- Requirements define what; this document defines how.
- Keep the design bounded to the selected vertical slice.
- Define components by responsibility, not speculative abstractions.
- Define interfaces in domain terms and include inputs, outputs, and errors.
- Show data and state flow, including success, failure, rollback, and recovery paths.
- Record security, performance, operational, migration, and compatibility implications.
- Consider alternatives and explain why the selected approach fits the approved constraints.
- A consequential or durable decision requires an ADR under `specs/adr/` with the next `ADR-NNN` ID and explicit alternatives and consequences.
- Do not introduce a crate, workspace member, architectural layer, or dependency without the constitution's required human approval.
- When Rust is involved, design for the dependency direction, port boundaries, typed errors, tests-first loop, and tooling gates in `specs/CONSTITUTION.md`.
- If implementation reveals that the requirements are wrong, stop and revise the requirements before revising the design.

## Output

Write the sibling `design.md` using `specs/templates/feature/design.md` when
available. Preserve the existing design ID when revising; use the next
feature-local `DES-NNN` value for a new artifact. Link `REQ-NNN` and all related
ADRs in front matter.

The file must contain:

- Context and constraints.
- Proposed design.
- Components and responsibilities.
- Interfaces and contracts.
- Data and state flow.
- Security, performance, and operations.
- Alternatives considered.
- Risks and open decisions.
- Verification approach.

Set `status: draft` while the design is under review. Do not create
`tasks.md` or implementation code as a side effect.

## ADR Handling

When a consequential decision is needed:

1. State the decision and the user-visible or architectural reason.
2. List credible alternatives and tradeoffs.
3. Ask for clarification when the decision is not already approved.
4. Create or revise the ADR only with explicit confirmation.
5. Link the ADR from `design.md` and keep its status consistent with its review state.

Do not treat a template as an ADR, and never reuse an ADR ID. The independent
`ADR-NNN` sequence includes active, archived, and superseded records.

## Review And File Writes

1. Synthesize the proposed technical approach, interfaces, state flow, risks, and decisions in chat.
2. Identify any blocking decisions and stop if one remains unresolved.
3. Ask whether to write the design as a draft, revise it, or approve the design and its ADRs.
4. Never overwrite an existing `design.md` or ADR silently; inspect it and require explicit confirmation before revising it.
5. After writing, report the design ID, ADR IDs, status, deferred decisions, and verification approach.

## Completion Checklist

- Story, scenarios, and requirements are approved and mutually consistent.
- The design is bounded to the selected slice.
- Interfaces, errors, state transitions, and recovery behavior are explicit.
- Every consequential decision is recorded or explicitly identified as deferred.
- Related ADRs are linked and their statuses are accurate.
- No new behavior or unapproved dependency has been introduced.
- Rust constraints from `specs/CONSTITUTION.md` are reflected when applicable.
- Verification covers unit, integration, acceptance, and relevant non-functional checks.
- No tasks or code were created as a side effect.
