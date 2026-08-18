# Multi-Agent SDD Workflow Plan

## Objective

Introduce an OpenCode-native multi-agent control plane around the repository's
existing specification-driven development workflow. Preserve human semantic
approval, keep each worker's context isolated, and make workflow dispatch
depend on explicit, machine-readable artifact state.

The workflow starts through an opt-in `/sdd` command. OpenCode does not run
background agents, so each invocation inspects the repository and resumes the
next legal action.

## Agent Topology

```text
User
  |
  v
sdd-coordinator (primary)
  |
  +-- sdd-designer (subagent)
  |     |
  |     +-- sdd-interviewer (hidden subagent)
  |
  +-- sdd-developer (subagent)
        |
        +-- sdd-coder (hidden subagent)
```

Set `subagent_depth: 2`. Task permissions strictly limit each parent to its
designated children.

| Agent | Responsibility | Write access |
| --- | --- | --- |
| `sdd-coordinator` | Inspect workflow state, select the next legal action, relay outcomes, and own final status transitions | Specification metadata only |
| `sdd-designer` | Order epics, select one story slice, and create the complete specification packet | `specs/**` |
| `sdd-interviewer` | Gather context, interview the user one question at a time, and produce one approved user story | Selected `user-story.md` |
| `sdd-developer` | Find implementation-ready stories, supervise coding, review results, and update task evidence | Selected `tasks.md` only |
| `sdd-coder` | Implement one approved packet test-first and run required gates | Code and tests, never specifications |

## Coordinator Dispatch

On every `/sdd` invocation, the coordinator will:

1. Read `AGENTS.md`, `SDD_WORKFLOW.md`, the approved PRD, artifact metadata,
   and relevant packet statuses.
2. Resume an implementation already in progress if one exists.
3. Dispatch the developer when exactly one story is implementation-ready.
4. Otherwise dispatch the designer to select the next dependency-eligible
   epic.
5. Stop if state is contradictory, dependencies are unresolved, or human
   approval is required.
6. Never run designer and developer writers concurrently against the shared
   worktree.

The coordinator must never interpret an OpenCode tool permission response as
semantic approval of a PRD, story, requirement, design, ADR, task plan, or
dependency decision.

## Epic And Story Selection

The designer topologically orders epics from explicit `depends_on` metadata.
PRD order is the deterministic tie-breaker when multiple epics are eligible.

For the selected epic, the designer identifies one independently valuable
vertical slice and launches one clean-context interviewer. The interviewer
refines only that story slice, not the epic's entire backlog.

After the story is approved, the designer owns the remaining specification
sequence:

```text
user-story.md
  -> scenarios.feature
  -> requirements.md
  -> design.md
  -> required ADRs
  -> tasks.md
  -> packet review
```

An epic becomes `done` only after explicit confirmation that no required slices
remain. Implementing one story does not implicitly complete its parent epic.

## Clean-Context Contract

Every child invocation begins by gathering its own context rather than relying
on parent conversation history. The parent prompt identifies only the role,
selected PRD, epic or packet, objective, and required output contract.

Each child independently reads:

- Repository instructions and workflow authority.
- Its complete input artifacts.
- Referenced ADRs and schema documents.
- Existing implementation where technically relevant.
- The complete Rust constitution before Rust work.
- Current worktree state before editing.

`question: allow` is enabled for the interviewer and designer so they can
interact with the user directly. Nested questions must be smoke-tested against
the installed OpenCode runtime. If child questions do not surface reliably,
the coordinator relays questions without changing workflow semantics.

## Role Boundaries

### Coordinator

The coordinator is the only primary agent in this workflow. It may invoke only
`sdd-designer` and `sdd-developer`. It owns workflow selection, validates child
summaries, obtains final semantic approvals, and applies final status
transitions.

### Designer

The designer may invoke only `sdd-interviewer`. It may write specifications but
not application code. It uses the existing Gherkin, requirements, design, ADR,
and task skills in their approved order. It must stop on unresolved product,
domain, dependency, behavior, or technical questions.

### Interviewer

The interviewer cannot invoke another agent. It asks one question at a time,
uses `refine-user-stories`, and writes one story only after explicit user
confirmation. It returns control after the story is approved or blocked.

### Developer

The developer may invoke only `sdd-coder`. It scans all active packets using
the readiness predicate, selects one eligible story according to dependencies
and epic order, and independently reviews the coder's result. It may update
task execution and verification evidence but cannot change approved behavior.

### Coder

The coder cannot invoke another agent or edit specifications. It implements
one complete approved packet test-first. If implementation reveals a behavior,
architecture, dependency, or specification gap, it stops and returns the
blocker rather than inventing a resolution.

## Explicit Workflow State

The current human-readable statuses are not sufficient for autonomous
dispatch. Rules, templates, and active artifacts need these machine-readable
elements:

| Location | Additions |
| --- | --- |
| PRD epic table | `Status` and `Depends On` |
| Feature Markdown frontmatter | Typed `depends_on`, `requires`, and `blockers` |
| `scenarios.feature` comments | `parent` and `status` |
| `tasks.md` | Packet-review status and verification evidence |
| Open questions | Normalized blocking stage and exact status enums |
| Completion records | Commands, observed result, revision, and timestamp or run reference |

### Scheduling References

Use separate fields for separate meanings:

```yaml
depends_on: [US-006]
requires: [ADR-006, DB-001, TABLE-007]
related: []
blockers: []
```

`depends_on` controls story scheduling. `requires` lists mandatory supporting
artifacts. `related` remains informational and never controls readiness.

### Blockers

Blockers use normalized categories, stages, and statuses:

```yaml
blockers:
  - id: OQ-001
    category: behavior
    stage: design
    blocking: true
    status: open
    owner: Quentin
    resolution: null
```

Allowed categories are `product`, `domain`, `dependency`, `behavior`, and
`technical`. Allowed statuses are `open`, `resolved`, and `waived`. A blocking
open item applies at its declared stage and every later stage.

### Scenario Metadata

Gherkin files use comments because they do not support YAML frontmatter:

```gherkin
# parent: US-010
# status: approved
```

Allowed scenario statuses match the repository lifecycle: `draft`,
`in-review`, `approved`, `implemented`, `archived`, and `superseded`.

## Implementation Readiness

A story is implementation-ready only when every condition below holds:

```text
parent PRD is approved
story is approved
scenarios are approved
requirements are approved
design is approved
tasks are approved
all required ADRs and schemas are approved
all story dependencies are implemented
no applicable blocker is open
no normative TBD or template marker remains
all references resolve
cross-artifact packet review is approved
current-session constitutional approvals are present
```

A coder is not dispatched merely because files exist or most statuses are
approved. Contradictory artifacts produce a blocked result.

## Implementation Completion

Implementation is complete only when every condition below is observed:

```text
all ordered tasks are checked
all verification-plan checks are checked
all Definition of Done checks are checked
red and green test evidence is reported
executable scenarios pass
cargo xtask ci is observed passing
delivered behavior matches approved specifications
no unresolved blocker or specification deviation remains
```

Only the coordinator transitions the packet to `implemented`, after the
developer reviews the coder's evidence. Successful tests alone do not authorize
a specification status transition.

## Completion Summary Contract

Every subagent returns the same structured envelope:

```text
Outcome: completed | blocked | needs-approval | failed
Role:
Phase:
Feature:
Inputs read:
Input statuses:
Files created:
Files modified:
Approvals consumed:
Approvals required:
Blocking questions:
Verification executed:
Observed results:
Residual risks:
Recommended next action:
```

Coder summaries additionally report:

```text
Red tests observed:
Green tests observed:
Constitution gates executed:
Specification deviations:
Working-tree limitations:
```

The authoritative summary contract belongs in `SDD_WORKFLOW.md`. Agent prompts
and skills reference it rather than duplicating it.

## Planned Files

### Create

- `opencode.json`
- `.opencode/agents/sdd-coordinator.md`
- `.opencode/agents/sdd-designer.md`
- `.opencode/agents/sdd-interviewer.md`
- `.opencode/agents/sdd-developer.md`
- `.opencode/agents/sdd-coder.md`
- `.opencode/commands/sdd.md`
- `.agents/skills/implement-feature/SKILL.md`
- `.agents/skills/verify-feature/SKILL.md`

### Revise

- `AGENTS.md`
- `README.md`
- `SDD_WORKFLOW.md`
- `.agents/skills/refine-user-stories/SKILL.md`
- `.agents/skills/user-story-to-gherkin/SKILL.md`
- `.agents/skills/create-requirements/SKILL.md`
- `.agents/skills/create-design/SKILL.md`
- `.agents/skills/create-tasks/SKILL.md`
- `specs/templates/project/prd.md`
- `specs/templates/feature/user-story.md`
- `specs/templates/feature/scenarios.feature`
- `specs/templates/feature/requirements.md`
- `specs/templates/feature/design.md`
- `specs/templates/feature/tasks.md`
- Supporting ADR templates where typed requirements or blockers apply.

## Existing Artifact Migration

Migrate active artifacts conservatively. Do not fabricate approval or
verification evidence.

`specs/010-semantic-index/` must not be dispatched automatically until its
cross-artifact conflicts are resolved. Known conflicts include passage
identity, supported vector dimensions, model-switch failure behavior, empty
collections, and requirement references.

Packets 007 through 009 claim `implemented` while their verification-plan
boxes remain unchecked. Preserve their historical status, but record missing
evidence as audit debt rather than treating it as observed verification.

Current unrelated worktree changes must not be reverted or overwritten during
migration.

## Implementation Sequence

1. Add the explicit state and readiness contract to `SDD_WORKFLOW.md` and role
   authority rules to `AGENTS.md`.
2. Update templates so new artifacts carry deterministic scheduling, blocker,
   approval, and verification metadata.
3. Update existing SDD skills to follow the state transitions and shared
   completion-summary contract.
4. Add repository-local implementation and verification skills governed by
   the Rust constitution.
5. Add OpenCode configuration, the five agent definitions, and the `/sdd`
   command with least-privilege permissions.
6. Smoke-test nested direct questions before relying on interviewer-to-user
   interaction.
7. Migrate existing PRD and feature metadata without inventing historical
   evidence.
8. Exercise the workflow against blocked, draft, ready, failed, and completed
   fixtures.
9. Document operation and troubleshooting in `README.md`.

## Verification Plan

1. Validate `opencode.json` against `https://opencode.ai/config.json`.
2. Run `opencode debug config`, `opencode agent list`, and agent-specific debug
   commands.
3. Confirm `subagent_depth` is exactly `2`.
4. Confirm the coordinator can spawn only designer and developer.
5. Confirm the designer can spawn only interviewer.
6. Confirm the developer can spawn only coder.
7. Confirm nested interviewer questions reach the user.
8. Test denied writes for every role boundary in a disposable fixture or
   worktree.
9. Exercise missing, draft, blocked, approval-required, ready,
   failed-verification, and completed packet states.
10. Confirm blocked agents never report `completed`.
11. Confirm commands not executed are never reported as passing.
12. Verify the setup still loads with `opencode --pure`.
13. Restart OpenCode after configuration changes because configuration is not
    hot-reloaded.

## Initial Defaults

- Use the invoking primary agent's model initially; do not add per-agent model
  policy until there is evidence it is useful.
- Serialize all writers because child sessions share the same worktree.
- Keep interviewer and coder hidden from normal autocomplete.
- Require explicit human semantic approval at every existing approval gate.
- Prefer deterministic blocking over inference when metadata is missing or
  contradictory.
