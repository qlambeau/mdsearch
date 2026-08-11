---
name: create-requirements
description: >-
  Create externally observable feature requirements from an approved user story
  and its Gherkin scenarios, resolving ambiguity before writing the contract.
---

# Create Requirements

## Purpose

This skill turns an approved user story and executable scenarios into a precise
requirements contract. It defines observable inputs, outputs, validation,
errors, invariants, and quality requirements. It does not define architecture,
database schemas, or implementation sequencing.

## Invocation

Examples:

- `Create requirements for specs/001-create-collection/`.
- `Create requirements from specs/001-create-collection/user-story.md`.
- `Complete the requirements for US-001 and its scenarios.`

Use the current working directory as the target project unless the user gives an
explicit project path. Resolve all paths relative to that root.

## Prerequisites

1. Resolve the target feature directory under `specs/NNN-feature-slug/`.
2. Read the complete approved `user-story.md`.
3. Read the sibling `scenarios.feature`.
4. Read the parent PRD and any related ADRs or project context.
5. Read `specs/CONSTITUTION.md` when the feature will eventually produce Rust code; do not rely on a summary.
6. Inspect whether `requirements.md` already exists.

Stop without writing when the story is not approved, the scenarios are absent,
or the story, scenarios, PRD, and existing decisions contradict one another.
Do not invent behavior to make the requirements complete.

## Contract Rules

- Requirements describe externally observable behavior, not implementation.
- Define CLI commands, arguments, switches, outputs, validation, and semantic failures when they are part of the approved behavior.
- Define postconditions, invariants, atomicity, persistence, and quality constraints explicitly.
- Preserve the story's scope boundaries and do not pull in future epics.
- Every functional requirement must trace to the source story and one or more named Gherkin scenarios.
- Every important Gherkin scenario must be covered by at least one functional requirement.
- Keep exact wording flexible when the story specifies a semantic output contract rather than stable text.
- Record technical choices as dependencies or deferred decisions; do not choose architecture in this file.

## Output

Write the sibling `requirements.md` using `specs/templates/feature/requirements.md`
when available. Preserve the feature's existing requirement ID when revising;
use the next feature-local `REQ-NNN` value for a new artifact. Set `parent` to
the story ID and link related artifacts.

The file must contain:

- Purpose and actors.
- Preconditions.
- Inputs, outputs, and validation.
- Functional requirements with priorities and traceability.
- Postconditions and invariants.
- Edge and failure behavior.
- Quality requirements.
- Dependencies and deferred decisions when applicable.
- Traceability to the story, scenarios, and parent PRD.

Set `status: draft` while the contract is under review. Do not create
`design.md`, `tasks.md`, code, or empty downstream files as a side effect.

## Review And File Writes

1. Synthesize the proposed contract and its traceability in chat.
2. List unresolved questions and identify which, if any, block the contract.
3. Ask whether to write the artifact as a draft, revise it, or stop for clarification.
4. Never overwrite an existing `requirements.md` silently; inspect it and require explicit confirmation before revising it.
5. After writing, report the path, requirement ID, status, covered scenarios, and remaining non-blocking decisions.

## Completion Checklist

- The source story is approved.
- The scenarios are present and behaviorally unambiguous.
- No blocking product or behavior question remains.
- Inputs, outputs, validation, errors, invariants, and quality requirements are explicit.
- No implementation-only decisions have leaked into the contract.
- Every functional requirement traces to story behavior and scenarios.
- Every scenario is covered by at least one requirement.
- Normative sections contain no unresolved `TBD` values.
- No downstream files or code were created.
