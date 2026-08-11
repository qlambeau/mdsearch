---
name: refine-user-stories
description: >-
  Refine one user story from a selected PRD epic through an adaptive,
  one-question-at-a-time interview, then write the confirmed story into a
  numbered feature directory without generating Gherkin or task specs.
---

# Refine User Stories

## Purpose

This skill performs just-in-time story discovery below an approved project or
major-feature PRD. It produces a business-focused story file that can later be
formulated into Gherkin by the `user-story-to-gherkin` skill.

## Invocation

Examples:

- `Refine a user story for EPIC-001 in specs/prds/PRD-001.md.`
- `Refine one story for the saved delivery addresses epic.`
- `Refine three stories for PRD-002, starting with the highest-value slice.`

Use the current working directory as the target project unless the user gives an
explicit project path. Accept an explicit epic ID or title, and accept an
optional story count or scope. If no count or scope is supplied, default to one
story.

## Prerequisites

1. Resolve the target root from the explicit path or current working directory.
2. Locate an applicable PRD under `specs/prds/`, or use an explicitly selected
   PRD path.
3. If no applicable PRD exists, stop without writing files and tell the user to
   run `create-prd` first.
4. If no epic was supplied, list the epics from the applicable PRD and ask the
   user to choose one.
5. If the requested epic cannot be found, stop without writing files and ask
   the user to identify an existing epic or revise the PRD.

Do not invent a PRD, epic, persona, business rule, or dependency to make the
skill continue.

Optional `specs/glossary.md`, `specs/product.md`, `specs/CONSTITUTION.md`,
`specs/tech.md`, and `specs/context.md` files may provide context when present,
but none is required by this skill. QMD is optional and must not be a runtime
dependency.

## Story Scope And Output

Produce one independently valuable, INVEST-shaped story per run by default. If
the user requests multiple stories, keep each story independently scoped and
preview each result before writing it.

For each approved story, scan existing active `specs/NNN-*/` directories and
archived or superseded `specs/archive/NNN-*/` directories and story metadata,
then use the next available sequential number. The default mapping is
`specs/001-feature-slug/user-story.md` with `id: US-001`. Keep the ID stable if
the title or slug changes. Allow an explicit ID only when the user is importing
an existing story and confirm collisions instead of reusing an ID.

User-story IDs use an independent monotonically increasing `US-NNN` sequence.
The numeric portion is exactly three zero-padded decimal digits. Template
placeholders such as `US-NNN` do not consume numbers.

The story file must contain:

- Story card: actor, goal, and value.
- Context and value.
- Business rules.
- Concrete examples.
- Concise, testable acceptance criteria as bullets or tables.
- In-scope and out-of-scope boundaries.
- Dependencies.
- Open and blocking questions.
- An INVEST check.

Set `parent` to the source PRD ID and record the source epic. Use the canonical
`specs/templates/feature/user-story.md` template when available. Set `status: draft`
while synthesizing and `status: approved` only after explicit confirmation.

Do not generate `scenarios.feature`, `requirements.md`, `design.md`, or
`tasks.md`. Do not update the PRD or create a central story index.

## Adaptive Interview

- Ask exactly one question at a time and wait for the answer.
- Start from the selected PRD epic and ask only for the highest-value missing
  information.
- Clarify the actor, user goal, business value, scope boundaries, rules,
  examples, error paths, dependencies, and measurable expectations as needed.
- Use concrete examples and edge cases to expose ambiguity.
- Keep the story free of database schemas, class or method names, UI styling,
  and other implementation prescriptions.
- Apply INVEST throughout the interview, especially independence, small size,
  value, and testability.
- Distinguish normal open questions from blocking questions.
- Stop before writing when a blocking product, domain, dependency, or behavior
  question remains.

## Confirmation And File Writes

1. Synthesize the proposed story in the chat.
2. Show the proposed ID, path, parent PRD, source epic, story card, rules,
   examples, scope, dependencies, and unresolved questions.
3. Ask for explicit approval or requested changes.
4. Only after approval, create the numbered feature directory and
   `user-story.md`. Do not create empty downstream files.

Never overwrite a story silently. If the target path or ID exists, read the
existing artifact, explain whether this is a revision or collision, and require
explicit confirmation before patching or replacing it. Preserve existing
decisions and open questions unless the user changes them.

## Completion Checklist

Before writing, verify:

- The story traces to a real PRD and epic.
- The actor, goal, and value are explicit.
- The story is independently valuable and small enough for roughly 1 to 3 days.
- Rules, examples, acceptance criteria, and boundaries are testable.
- Implementation details have not leaked into the story.
- Blocking questions are resolved.
- The user explicitly approved the synthesized story.

Report the created path, story ID, parent PRD, source epic, and any remaining
non-blocking open questions after writing.
