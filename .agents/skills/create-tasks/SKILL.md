---
name: create-tasks
description: >-
  Create an ordered implementation and verification plan from an approved
  feature design, requirements contract, scenarios, ADRs, and Rust constitution.
---

# Create Tasks

## Purpose

This skill turns an approved feature packet into actionable implementation work.
Tasks describe order, dependencies, completion checks, testing, rollout, and
recovery. They must not introduce new behavior or silently revise the design.

## Invocation

Examples:

- `Create tasks for specs/001-create-collection/`.
- `Break the approved US-001 design into implementation tasks.`
- `Complete tasks.md for this feature packet.`

Use the current working directory as the target project unless the user gives an
explicit project path. Resolve all paths relative to that root.

## Prerequisites

1. Resolve the target feature directory under `specs/NNN-feature-slug/`.
2. Read the complete approved `user-story.md`, `scenarios.feature`, `requirements.md`, and `design.md`.
3. Read all related approved ADRs and the complete `specs/CONSTITUTION.md`.
4. Inspect the current codebase and available tooling when implementation code exists.
5. Inspect whether `tasks.md` already exists.

Stop without writing when the design or requirements are not approved, a
blocking decision remains, or the task breakdown would require inventing
behavior. Ask for clarification rather than hiding a specification gap in a
task.

## Task Rules

- Use one independently verifiable vertical feature slice.
- Order tasks by dependency, not by technical layer alone.
- Each task must state a concrete outcome, dependencies, and a verification check.
- Include test-first work and make the red-to-green path observable.
- Cover unit, integration, CLI, Gherkin, failure, recovery, and relevant non-functional checks.
- Keep tasks within the approved scope; explicitly exclude future epics and speculative refactors.
- Do not add a dependency, workspace member, architectural layer, or public API not justified by the approved design and constitution.
- For Rust, include the constitution's required gates and Definition of Done checks, including observed output from `cargo xtask ci` when the workspace exists.
- Rollout and recovery must explain migration, retry, rollback, and partial-failure behavior.
- Tasks do not change requirements or design. Revise those artifacts first when behavior changes.

## Output

Write the sibling `tasks.md` using `specs/templates/feature/tasks.md` when
available. Preserve an existing task ID when revising; use the next
feature-local `TASK-NNN` value for a new artifact. Link the parent story,
requirements, design, and related ADRs in front matter.

The file must contain:

- Implementation approach.
- Ordered tasks with dependencies and verification.
- Test and verification plan.
- Rollout and recovery.
- Definition of Done.

Set `status: draft` while the plan is under review. Do not create code, issue
tracker records, or empty downstream files as a side effect.

## Review And File Writes

1. Synthesize the task sequence, dependencies, scope exclusions, and checks in chat.
2. Identify any task that depends on an unresolved decision and stop if it blocks the plan.
3. Ask whether to write the task plan as a draft, revise it, or approve it.
4. Never overwrite an existing `tasks.md` silently; inspect it and require explicit confirmation before revising it.
5. After writing, report the task ID, task count, dependency order, verification coverage, and remaining non-blocking work.

## Completion Checklist

- Story, scenarios, requirements, design, ADRs, and constitution context were read.
- Every task has a concrete outcome, dependency list, and verification check.
- The dependency graph is ordered and acyclic.
- Tests are planned before implementation and cover every specified behavior and error path.
- Gherkin scenarios are included in the verification plan.
- Rust constitution gates are included when Rust code is in scope.
- Rollout and recovery behavior are explicit.
- No task changes approved behavior or design.
- No code or issue tracker records were created as a side effect.
