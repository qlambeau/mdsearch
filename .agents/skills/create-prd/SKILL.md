---
name: create-prd
description: >-
  Create or revise a project or major-feature Product Requirements Document
  through a one-question-at-a-time interview, then write it only after explicit
  user confirmation.
---

# Create PRD

## Purpose

This skill is the project entry point for the SDD workflow. It turns an initial
idea into a structured Product Requirements Document without inventing facts.
It is a product-framing skill, not a technical design or implementation-plan
generator.

## Invocation

Examples:

- `Create a PRD for a new project that helps customers save delivery addresses.`
- `Create a major-feature PRD for recurring billing in this project.`
- `Revise the project PRD using the new business constraints.`

Use the current working directory as the target project unless the user gives an
explicit project path. Resolve all output paths relative to that target root.

## Scope And Output

Support two explicit scopes:

1. **Project PRD**: write `specs/prds/PRD-NNN.md`, using the next available PRD
   ID.
2. **Major-feature PRD**: write `specs/prds/PRD-NNN.md`, using the next
   available PRD ID. The project PRD is the parent when one exists.

All PRDs use the same independent monotonically increasing `PRD-NNN` sequence.
The numeric portion is exactly three zero-padded decimal digits. Scan active,
archived, and superseded artifacts before allocating an ID; never reuse an ID.
Template placeholders such as `PRD-NNN` do not consume numbers.

The project PRD and major-feature PRDs use the same required sections:

1. Vision and problem.
2. Target personas and journeys.
3. Success metrics.
4. Functional scope and epics.
5. Non-functional requirements.
6. Assumptions and out-of-scope boundaries.
7. Open questions.
8. Decision log.

Use the canonical `specs/templates/project/prd.md` template when it is
available.
Keep the front matter fields and section names stable. Set `status: draft`
while synthesizing and `status: approved` only after the user confirms the
final draft.

## Prerequisite And Context Discovery

1. Resolve the target root from the explicit path or current working directory.
2. Read existing `specs/` artifacts and lightweight project context such as
   `README.md` when they exist.
3. Scan `specs/prds/` and any archived or superseded PRDs for ID collisions.
4. Check whether the intended output already exists.
5. If the user did not provide an initial idea, ask for the product or feature
   premise before beginning the structured interview.

QMD and the personal wiki are optional. Use them only when the user explicitly
requests background research and they are available. Never make the skill fail
because QMD or the wiki is absent.

## Interview Rules

- Ask exactly one question at a time and wait for the answer.
- Adapt the next question to the highest-value ambiguity; do not run a rigid
  questionnaire when the answer is already known.
- Cover the six required PRD areas, then inspect assumptions, boundaries,
  decisions, and open questions.
- Ask for concrete users, journeys, outcomes, and measurable targets rather
  than accepting vague goals such as "make it better."
- Keep epics at capability level. Do not prematurely write user stories,
  database schemas, class names, API method signatures, or task lists.
- Distinguish a normal `Open Question` from a `Blocking Question`.
- Do not infer business facts, constraints, metrics, or personas. Record
  unknowns as `TBD` or open questions instead.

The interview may finish with non-blocking open questions. Stop and continue
questioning when the problem, intended users, or scope cannot be stated well
enough to make the PRD meaningful.

## Confirmation And File Writes

1. Synthesize the complete PRD in the chat.
2. Show the user the proposed path, metadata, required sections, assumptions,
   and all open questions.
3. Ask for explicit approval or requested changes.
4. Write the file only after approval. Create `specs/` or `specs/prds/` as
   needed, but do not create empty feature directories or other project files.
5. Set `updated` to the current date and preserve the approved content.

Never overwrite a file silently. If the target exists, read it first and tell
the user whether the operation is a revision. Require explicit confirmation
before replacing or patching it, and preserve existing decisions and unresolved
questions unless the user changes them.

## Completion Checklist

Before writing, verify:

- The scope and title are explicit.
- The six required sections are present.
- Personas, journeys, epics, and outcomes are concrete enough to discuss.
- Success metrics are measurable or marked `TBD`.
- Assumptions and out-of-scope boundaries are explicit.
- Blocking questions are resolved or the interview has stopped.
- The PRD contains no feature-level implementation plan.
- The user explicitly approved the synthesized draft.

Report the created or revised path, PRD ID, scope, and any non-blocking open
questions after writing.
