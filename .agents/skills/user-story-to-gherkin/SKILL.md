---
name: user-story-to-gherkin
description: >-
  Translate a confirmed user story into a colocated Gherkin scenarios.feature
  file, resolving ambiguities with the user before writing executable behavior.
---

# User Story to Gherkin Translator

## Overview
This skill translates a confirmed user story from Markdown into one structured
Gherkin `Feature`, enabling BDD (Behavior-Driven Development). It is the
formulation step after `refine-user-stories`; it does not create or revise the
business story itself.

## Dependencies
None.

## Quick Start
Provide the agent with a file path to your user stories:
`Translate specs/001-saved-address/user-story.md to Gherkin`

## Workflow

### 1. Locate and Read the Input File
* Locate the specified user story file. If not found or if the path is not provided, ask the user for the file path.
* Read the complete story, including its business rules, examples, acceptance criteria, scope boundaries, dependencies, and open questions.
* If the path is under `specs/NNN-feature-slug/` and the filename is `user-story.md`, use the sibling `scenarios.feature` as the default output.
* For a story outside that layout, use an explicitly supplied output path. If none is supplied, use a sibling `scenarios.feature` rather than creating a central directory.
* Never silently replace an existing output file; inspect it and ask for confirmation before revising it.

### 2. Analysis and Ambiguity Check
* Inspect each story's Acceptance Criteria.
* Inspect the examples, business rules, dependencies, and open questions as well as the acceptance criteria.
* Identify missing boundary conditions, logic gaps, or ambiguities (e.g., unspecified error messages, undefined default values, unclear actors, or contradictory outcomes).
* **Mandatory Action:** If any issues or ambiguities are found, stop and ask the user for clarification before generating files.
* Do not resolve domain ambiguity by guessing or by adding implementation details.

### 3. Translation to Gherkin
* Map the user story to exactly one Gherkin `Feature` in the target file.
* Translate acceptance criteria into clear `Given-When-Then` Scenarios.
* Use `Scenario Outline` with `Examples` tables for parameterized behavior (e.g., input validation boundaries).
* Keep steps focused on behavioral intent (e.g., "When I create a task") rather than UI implementation details (e.g., "When I click the green submit button").
* Cover the confirmed happy path, important alternate paths, and relevant failure or boundary behavior.
* Use the project's ubiquitous language when a glossary is available.

### 4. Output Generation
* Create the story's feature directory only when the input story is already in the canonical layout.
* Write one `scenarios.feature` file per story. Do not create one file per scenario.
* Keep the story and its executable contract colocated.
* List the created file and summarize the scenarios written for the user.

## Common Mistakes
* **Skipping the Ambiguity Check:** Proceeding with generation when acceptance criteria are vague or incomplete.
* **Over-complicating Scenarios:** Writing steps that specify UI implementation details instead of behavioral intent.
* **Splitting One Story Across Files:** Creating several feature files when one story should have one `scenarios.feature` contract.
* **Generating Empty Task Specs:** Creating `requirements.md`, `design.md`, or `tasks.md` as a side effect of Gherkin generation.
