# SDD Workflow Audit And Upgrade Plan

**Document status:** Proposed remediation roadmap
**Audit date:** 2026-08-21
**Scope:** Repository workflow guidance, skills, templates, active specification packets, ADRs, schema artifacts, and Rust verification integration

## Executive Assessment

The repository has a strong specification vocabulary and a well-intentioned
spec-first workflow. The Constitution, artifact boundaries, interview discipline,
and no-silent-overwrite rules are particularly strong.

The workflow is not currently closed as an executable control system. Approval
transitions are incomplete, completion statuses are not consistently supported by
observed evidence, the canonical workflow omits the mandatory red-test gate, and
the multi-agent readiness contract remains a plan rather than an implemented
capability. Active packets already contain contradictory public contracts and
unresolved decisions marked as approved. The workflow also ends at
implementation and archival: there is no defined post-release process for
observations, fixes, regressions, or release closure.

The immediate rule should be:

> Do not dispatch implementation or infer completion from frontmatter status alone
> until the lifecycle, evidence, and readiness contracts below are established.

This document records the audit findings and the recommended upgrade sequence. It
does not itself approve unresolved product behavior, backfill missing historical
evidence, or change the meaning of existing specifications.

## Audit Coverage

The audit inspected the following repository surfaces:

| Surface | Coverage |
| --- | --- |
| Workflow authorities | `AGENTS.md`, `SDD_WORKFLOW.md`, `docs/SDD_WORKFLOW_KIT.md`, `specs/CONSTITUTION.md` |
| Process guidance | `specs/MULTI_AGENT_SDD_PLAN.md`, `specs/prd_lifecycle_and_evolution_plan.md`, `specs/TODO.md` |
| Local skills | All seven files under `.agents/skills/`, including the six SDD skills and optional `qmd` skill |
| Templates | All ten files under `specs/templates/` |
| Active product specifications | `PRD-001`, feature packets `001` through `013` |
| Supporting specifications | `ADR-001` through `ADR-008`, `DB-001`, `TABLE-001` through `TABLE-012` |
| Verification integration | `README.md`, `xtask/src/main.rs`, repository CI/configuration presence |
| Post-release surfaces | `specs/TODO.md` observation catalogue, PRD lifecycle guidance on bugfixes, release references in `README.md` |

No implementation tests or builds were run during the audit. The existing
modification to `specs/TODO.md` was not changed.

## Confirmed Strengths

1. Artifact responsibilities are clearly separated between product intent,
   business behavior, executable scenarios, requirements, design, ADRs, tasks,
   and schema documents.
2. The Constitution provides unusually explicit rules for test-first work,
   traceability, typed errors, dependencies, layering, coverage, and completion.
3. PRD and story interviews require one question at a time and explicit approval
   before writing.
4. The local skills consistently prohibit silent overwrites and accidental
   creation of downstream placeholder artifacts.
5. All thirteen active feature directories contain the expected five packet files.
6. No duplicate formal IDs were found in the current active artifact set.
7. Database and table frontmatter references are bidirectional.
8. Illustrative delivery-address content from the templates was not found in the
   active feature packets.
9. QMD is correctly treated as optional and is not a runtime workflow dependency.

## Findings

### Critical Findings

#### F-001: Downstream approval transitions are undefined

**Evidence:**

- `docs/SDD_WORKFLOW_KIT.md:93-105` says only `create-prd` and
  `refine-user-stories` set `approved`.
- `create-requirements/SKILL.md:71-80` writes requirements as `draft` but gives no
  approved transition.
- `create-design/SKILL.md:73-95` and `create-tasks/SKILL.md:68-77` have the same
  gap.
- Design and task prerequisites require approved upstream artifacts at
  `create-design/SKILL.md:35-38` and `create-tasks/SKILL.md:35-38`.

**Impact:** The documented skill chain can create draft artifacts but cannot
reliably reach the spec-ready gate. In practice, status changes become manual,
undocumented, and impossible to distinguish from accidental edits.

**Required outcome:** Define explicit transitions, approver fields, approval
evidence, and the exact actor allowed to promote each artifact.

#### F-002: The canonical workflow omits the constitutional red-test gate

**Evidence:**

- The Constitution defines the fixed loop as `SPEC -> CONTRACT -> RED TEST ->
  IMPLEMENTATION -> GREEN -> REFACTOR -> TRACE` at `specs/CONSTITUTION.md:42-60`.
- Red-first testing is mandatory at `specs/CONSTITUTION.md:363-367`.
- `SDD_WORKFLOW.md:23-27` and `SDD_WORKFLOW.md:51-55` show implementation before
  verification.
- `AGENTS.md:30-33` also presents implementation before running tests.

**Impact:** The repository's primary workflow can be followed in a way that
violates a non-negotiable engineering rule.

**Required outcome:** Add explicit `RED_TESTS` and `GREEN_TESTS` gates between
packet approval and implementation, with observed evidence recorded in the task
artifact or completion record.

#### F-003: `implemented` status is not evidence-backed

**Evidence:**

- Packets `007`, `008`, `009`, and `010` are marked `implemented` while their
  verification plan remains unchecked.
- Packet `010` also has an unchecked Definition of Done at
  `specs/010-semantic-index/tasks.md:165-174`.
- The completion contract in `specs/MULTI_AGENT_SDD_PLAN.md:225-242` requires all
  tasks, verification checks, Definition of Done checks, red/green evidence,
  executable scenarios, and `cargo xtask ci` evidence.
- The plan acknowledges the `007`-`009` evidence debt at lines `321-323` but does
  not include the equivalent `010` status debt.

**Impact:** A coordinator or developer cannot distinguish observed completion from
historical claims. This defeats automated dispatch and makes audits unreliable.

**Required outcome:** Preserve historical statuses, but add explicit audit-debt
and evidence fields. Never fabricate missing test output or retroactively mark
unchecked gates as observed.

#### F-004: The declared Rust quality gate is incomplete and not CI-enforced

**Evidence:**

- `xtask/src/main.rs:78-118` runs six commands and enforces only workspace line
  coverage of 85%.
- The Constitution also requires domain coverage of 95% at
  `specs/CONSTITUTION.md:400-407`.
- Additional release, Miri-when-applicable, `cargo machete`, and `udeps` checks are
  required at `specs/CONSTITUTION.md:480-491` but are not implemented in `xtask`.
- `cargo xtask eval` is a separate command, although `ADR-004.md:59-66` requires
  evaluation regressions to fail CI.
- No repository CI workflow configuration was found.
- `README.md:533` documents `cargo test --workspace`, which omits the required
  `--all-features` form.

**Impact:** The documented Definition of Done can pass while required quality and
retrieval gates have not run.

**Required outcome:** Make one authoritative local gate and one authoritative CI
pipeline execute the same required checks, including evaluation thresholds and
the domain coverage floor.

### High Findings

#### F-005: The multi-agent readiness contract is aspirational

**Evidence:**

- Required machine-readable fields are described at
  `specs/MULTI_AGENT_SDD_PLAN.md:142-155`.
- The readiness predicate requires approved scenarios, tasks, ADRs, schemas,
  dependencies, blockers, references, packet review, and constitutional approval
  at lines `202-223`.
- The completion-summary contract is specified at lines `244-277`.
- The coordinator, role definitions, `/sdd` command, implementation skill, and
  verification skill are only listed as planned files at lines `279-309`.

**Impact:** The plan describes stronger controls than the active workflow and
artifacts provide. Treating it as active policy would create false confidence.

**Required outcome:** Either explicitly label the plan as future-state or implement
its state model, role boundaries, command, and verification fixtures before using
it for dispatch.

#### F-006: Approved-source and scenario approval gates are not enforced

**Evidence:**

- `refine-user-stories/SKILL.md:30-43` locates a PRD and epic but does not verify
  that the PRD is approved.
- `user-story-to-gherkin/SKILL.md:23-37` does not verify story approval or require
  explicit final confirmation before writing.
- `scenarios.feature` has no parent ID, status, or approval metadata.

**Impact:** Draft or superseded sources can produce downstream contracts that look
approved or executable.

**Required outcome:** Every downstream skill must validate exact upstream status,
blockers, and reference integrity before writing. Gherkin must have explicit
approval semantics even though it cannot use YAML frontmatter.

#### F-007: Feature 011 is approved with unresolved blocking behavior

**Evidence:**

- `specs/011-hybrid-search/user-story.md:183-188` leaves the re-ranker decision
  open and marks it blocking for design.
- `specs/011-hybrid-search/requirements.md:131-145` retains a normative latency
  `TBD` and defers decisions to design.
- `specs/adr/ADR-007.md:39-63` records decisions that were not reconciled into the
  story and requirements metadata.

**Impact:** The packet has an approved status while violating the documented
  spec-ready rule that blocking questions and normative `TBD` values must be
  resolved.

**Required outcome:** Reconcile the story, requirements, design, ADR, scenarios,
  and tasks. Do not dispatch until the blocker and normative target are resolved
  or formally reclassified as non-normative.

#### F-008: Feature 010 has an unresolved cross-artifact scope conflict

**Evidence:**

- The story excludes model selection and re-ranker behavior at
  `specs/010-semantic-index/user-story.md:148-155`.
- Requirements and design add `--reranker` behavior at
  `specs/010-semantic-index/requirements.md:55-69,93-95` and
  `specs/010-semantic-index/design.md:55-62`.
- Tasks do not include the corresponding implementation work at
  `specs/010-semantic-index/tasks.md:28-45`.
- ADR-007 explicitly says the existing contract must be revised before the
  extension at `specs/adr/ADR-007.md:103-109`.

**Impact:** The packet does not have a single authoritative behavior boundary.

**Required outcome:** Split the extension into the correct packet or update and
reapprove all affected artifacts before implementation status is considered valid.

#### F-009: Active packets disagree on public command and identity contracts

**Evidence:**

- Existing product documentation and feature 005 use `mdsearch collection update`
  at `README.md:127-231` and `specs/005-update-collection/user-story.md:37-46`.
- Feature 012 uses `mdsearch update` at
  `specs/012-entity-graph/user-story.md:52-58` and its scenarios.
- Feature 012 requirements define file nodes by stable `file_id` at
  `specs/012-entity-graph/requirements.md:68-70`.
- `TABLE-010.md:54-66` and `specs/013-context-recovery/design.md:61-63` define
  file-node identity by canonical path.

**Impact:** These are externally visible or persistence-level contracts. They can
produce incompatible code and database behavior even when each individual packet
looks internally coherent.

**Required outcome:** Select one command contract and one file-node identity model,
then update all affected stories, scenarios, requirements, designs, schemas, ADRs,
README content, and tests together.

### Medium Findings

#### F-010: Accepted ADR immutability conflicts with design skill behavior

The Constitution requires accepted ADRs to be immutable and superseded rather than
edited at `specs/CONSTITUTION.md:423-432`. The design skill says to “create or
revise” ADRs at `create-design/SKILL.md:76-87`. This can destroy decision history.

#### F-011: Identifier rules are incomplete and task IDs collide

`AGENTS.md:67-85` defines allocation rules for PRD, ADR, story, chart, database,
and table IDs but omits requirements, designs, and tasks. The task template uses
`TASK-001` both for the task document and its first task item at
`specs/templates/feature/tasks.md:1-2,25-32`.

#### F-012: Templates do not implement the complete skill contracts

The requirements template has no explicit dependencies/deferred-decisions section
or parent PRD traceability. The tasks template has no packet-review state,
completion evidence, or Constitution-gate evidence. The project PRD template uses
the ambiguous `scope: project-or-major-feature` and has no `supersedes` field.

#### F-013: Gherkin traceability and coverage are too weak to be executable gates

The scenario template has no story ID or lifecycle status. Requirements rows
usually trace to a story but not named scenarios. Existing scenarios omit some
required rollback, cycle protection, re-ranker provisioning, and operator
neutralization cases. The translator skill also provides no syntax or execution
validation step.

#### F-014: Schema artifacts contain drift and unenforced invariants

`TABLE-001.md:34-51` describes schema version 2 while `DB-001.md:69-78` is at
version 6. `TABLE-005.md:35-72` describes foreign-key and uniqueness invariants
that its DDL does not enforce. `TABLE-011.md:37-79` similarly describes
per-collection edge invariants not enforced by its DDL.

#### F-015: Status, ownership, and archival metadata are inconsistent

Features 012 and 013 have approved story/requirements/design artifacts but
implemented task artifacts. Many approved or implemented artifacts retain
`owner: TBD`, despite `AGENTS.md:91-97` requiring placeholder metadata to be
replaced before approval. `specs/archive/` contains only `.gitkeep` even though
the workflow defines archival of completed packets.

#### F-016: PRD lifecycle guidance is stale and not integrated with `create-prd`

The lifecycle plan refers to epics `EPIC-001..006` at
`specs/prd_lifecycle_and_evolution_plan.md:30-37`, while PRD-001 includes
`EPIC-007`. The lifecycle guide requires `supersedes` and superseded status
handling at lines `99-103`, but the PRD skill does not implement that procedure.
PRD-001 also retains stale statements about the golden set and entity extraction.

#### F-017: Task planning requires evidence that only exists after implementation

`create-tasks/SKILL.md:40-51` requires observed `cargo xtask ci` output while
creating the task plan. This mixes planning with execution evidence and can either
encourage false claims or block planning prematurely.

#### F-018: Exact-byte retrieval behavior conflicts with the design

Feature 009 promises exact stored bytes at
`specs/009-get-file/requirements.md:61-84`, while its design returns a `String`
and rejects non-UTF-8 content at `specs/009-get-file/design.md:112-124`.
The product must explicitly choose UTF-8-only Markdown or a byte-preserving output
contract.

### Post-Release Operations Findings

The following findings come from the post-release process review. They are part
of the same audit and are addressed by the same upgrade plan.

#### F-019: No post-release operations phase exists

**Evidence:**

- The canonical workflow ends at implementation, verification, review, and
  archival at `SDD_WORKFLOW.md:47-58` and `docs/SDD_WORKFLOW_KIT.md:33-36`;
  there is no defined loop for reports that arrive after a release.
- The PRD lifecycle guide addresses bugfixes only negatively at
  `specs/prd_lifecycle_and_evolution_plan.md:14,108-112`: they must not create a
  PRD, but no positive handling path is defined.
- `specs/TODO.md` is a static catalogue with no lifecycle, ownership, release
  origin, or closure evidence.

**Impact:** Post-release work has no defined path, so fixes and observations are
handled ad hoc, drift from the specification-first rule, or are never closed.

**Required outcome:** Add a documented post-release phase with observation
intake, triage, fix planning, verification, release closure, and audit trails.

#### F-020: Observations are not classified before work begins

**Evidence:**

- `specs/TODO.md:33-55` defines kinds and priorities but no classification
  decision that routes a report to a defect, regression, behavior change,
  technical debt, or documentation path.
- Every report therefore receives the same treatment or is deferred without a
  resolution path.

**Required outcome:** Make triage classification mandatory before any fix work,
with one handling path per class.

#### F-021: The fix contract is undefined

**Evidence:**

- Nothing distinguishes a fix for code that violates an approved contract from a
  fix that changes the contract itself.
- The Constitution's spec-first loop at `specs/CONSTITUTION.md:42-60` and the
  workflow rule to update and approve specifications before changing behavior at
  `AGENTS.md:32-33` are not operationalized for post-release fixes.
- No emergency or release-blocker path exists with explicit, non-waivable gates.

**Required outcome:** Define three fix paths — contract-violation fix,
specification amendment, and emergency release-blocker fix — each with its own
mandatory gates.

#### F-022: There is no durable release record

**Evidence:**

- `README.md:84-91` documents building and installing the binary, and ADR-004
  defines evaluation regression gates, but no artifact records what a release
  contains, which revision produced it, which gates passed, which observations
  were known, or how to roll back.

**Required outcome:** Every release produces a release record with version,
revision, included packets and fixes, gate evidence, known observations, and
rollback/recovery information.

#### F-023: Observation-to-closure traceability is missing

**Evidence:**

- `specs/TODO.md:31-42` defines a `PROMOTED` convention, but there is no chain
  from observation to triage, specification, fix, regression test, verification,
  release, and closure.
- The same evidence gap already affects packet statuses (`F-003`); without a
  closure evidence contract, observation and fix records will accumulate the
  same debt.

**Required outcome:** Define one mandatory trace from observation to closure and
validate it mechanically.

## Target State

The upgraded workflow should satisfy all of the following:

1. Every artifact has one authoritative lifecycle status and a durable approval
   record.
2. Every feature packet has a separate packet-level readiness and completion
   state; artifact statuses alone never authorize dispatch.
3. A draft source cannot generate an approved downstream artifact.
4. Every normative behavior maps to at least one named scenario and test.
5. Implementation begins only after the red-test gate has been observed.
6. Completion requires observed verification output, not checked boxes alone.
7. Accepted ADRs are append-only; changes create superseding ADRs.
8. Machine-readable blockers, dependencies, required artifacts, and references are
   validated before dispatch.
9. Local and CI quality gates execute the same commands and retrieval evaluation
   thresholds.
10. Existing historical evidence is preserved and missing evidence is recorded as
    debt rather than fabricated.
11. Every release produces a durable release record with version, revision,
    included packets and fixes, gate evidence, known observations, and rollback
    information.
12. Every observation or defect follows a classified triage path from intake to
    closure with one mandatory trace: observation, triage, specification, fix,
    regression test, verification, release, and closure.
13. Fixes never bypass the specification-first rule: contract-violation fixes,
    specification amendments, and emergency release-blocker fixes each have
    explicit non-waivable gates.
14. `specs/TODO.md` serves as the observation index while full observation and
    release records live in dedicated artifacts.

## Recommended Upgrade Plan

### Phase 0: Establish Authority And Freeze Unsafe Dispatch

**Objective:** Stop contradictory documentation from being interpreted as an
active autonomous control plane.

**Actions:**

1. Declare `specs/CONSTITUTION.md` authoritative for Rust engineering rules.
2. Declare `SDD_WORKFLOW.md` authoritative for the base lifecycle until the
   multi-agent plan is implemented.
3. Mark `specs/MULTI_AGENT_SDD_PLAN.md` as future-state in its own document or
   implement its missing control-plane files before enabling dispatch.
4. Prohibit implementation dispatch based solely on `status: approved` or
   `status: implemented`.
5. Create a decision record defining whether the repository will adopt the
   multi-agent plan in this upgrade or retain the single-agent workflow.
6. Preserve the current `specs/TODO.md` worktree modification and all historical
   packet statuses during migration.

**Deliverables:** One authority map, one dispatch rule, one decision on the
multi-agent plan, and an explicit audit-debt policy.

**Exit gate:** A reviewer can identify exactly which document controls each
workflow decision and can explain why no current packet is automatically
dispatchable without readiness validation.

### Phase 1: Define The Lifecycle And State Contract

**Objective:** Make approval, readiness, implementation, verification, and
archival transitions explicit and machine-readable.

**Actions:**

1. Define the shared lifecycle:
   `draft -> in-review -> approved -> implemented -> archived`.
2. Define `superseded` as a terminal state requiring a replacement reference.
3. Define artifact-level status separately from packet-level status.
4. Define who may promote each artifact and what evidence is required.
5. Define blocker categories and stages: `product`, `domain`, `dependency`,
   `behavior`, and `technical`.
6. Define normalized blocker statuses: `open`, `resolved`, and `waived`.
7. Define packet readiness as a conjunction of source approval, artifact approval,
   dependency completion, blocker resolution, placeholder removal, reference
   resolution, scenario coverage, and packet review.
8. Define completion as a conjunction of ordered task checks, verification checks,
   Definition of Done checks, red/green evidence, scenario results, CI results,
   specification match, and no unresolved deviation.
9. Define the completion record fields: command, observed result, revision, date,
   actor, and run reference.
10. Define current-session human approval handling for new dependencies,
    workspace members, architectural layers, and constitutional exceptions.
11. Define the observation lifecycle: `new -> triaged -> investigated ->
    confirmed | rejected | duplicate | deferred -> planned -> ready ->
    implementing -> verified -> released -> closed`.
12. Define the fix packet contract: the observation it resolves, the affected
    approved specification, the fix class (contract violation, specification
    amendment, or emergency), the regression test, and the verification
    evidence.
13. Define the release record contract: version, source revision, included
    packets and fixes, executed gates with observed results, known open
    observations, and rollback/recovery instructions.

**Recommended metadata shape:**

```yaml
status: approved
owner: Quentin
parent: US-001
depends_on: []
requires: [ADR-001, DB-001]
blockers: []
approval:
  state: approved
  approved_by: Quentin
  approved_at: 2026-08-21
  approval_ref: session-or-review-reference
packet_review: approved
verification:
  state: not-run
  records: []
```

The exact field names should be finalized in the state decision before template
migration. `related` must remain informational and must not control readiness.

**Observation record shape (for reference):**

```yaml
# Observation record (specs/observations/OBS-NNN.md)
id: OBS-014
title: "Lexical and hybrid search treat the same query string differently"
class: behavior-inconsistency
severity: high
reported: 2026-08-21
reported_by: reviewer
release_origin: v0.1.0
status: confirmed
owner: Quentin
blocking: false
linked_spec: [US-007, US-011]
resolution: null
closed: null
```

Observation records must keep the original report immutable and record decisions
and evidence as append-only fields. Never rewrite the original observation in
place.

**Deliverables:** State-transition table, readiness predicate, completion predicate,
approval/evidence schema, and blocker schema.

**Exit gate:** A fixture can be classified unambiguously as missing, draft,
blocked, approval-required, ready, failed-verification, completed, superseded, or
archived.

### Phase 2: Align Workflow Documentation

**Objective:** Remove contradictory instructions from the human-readable workflow.

**Actions:**

1. Update `AGENTS.md` so story refinement starts from an approved PRD, not an
   approved story.
2. Add the red-test gate and explicit verification evidence to `SDD_WORKFLOW.md`.
3. Add requirements approval, scenario approval, task approval, and packet review
   as visible gates.
4. Add the completion-summary contract to `SDD_WORKFLOW.md` if the multi-agent
   design remains active.
5. Add schemas and charts to the implementation packet when applicable.
6. Align `docs/SDD_WORKFLOW_KIT.md` with the authoritative state and transition
   model.
7. Update the PRD lifecycle guide for `EPIC-007` and integrate its supersession
   rules into `create-prd`.
8. Reconcile stale Constitution workspace examples with the already-existing Rust
   workspace, while preserving the Constitution's engineering rules.
9. Update README development commands to match the authoritative all-features and
   evaluation gates.
10. Add a `Post-Release Operations` phase to `SDD_WORKFLOW.md`, `AGENTS.md`, and
    `docs/SDD_WORKFLOW_KIT.md` covering observation intake, triage, fix
    planning, the emergency path, and release closure.
11. Convert `specs/TODO.md` into the observation index with a defined lifecycle,
    or move full records to `specs/observations/OBS-NNN.md` and keep `TODO.md`
    as the index of unresolved and deferred items.
12. Document the emergency and release-blocker fix path with non-waivable gates:
    human approval, a regression test, a specification reference, and recorded
    verification evidence.

**Deliverables:** Consistent `AGENTS.md`, `SDD_WORKFLOW.md`, workflow-kit docs,
PRD lifecycle guide, Constitution references, and README commands.

**Exit gate:** A new contributor following only the read-first documents cannot
accidentally skip approval, red tests, packet review, or required CI gates.

### Phase 3: Upgrade Skills And Templates

**Objective:** Make each skill enforce its handoff contract and produce complete
artifacts.

**Skill changes:**

1. `create-prd`: validate project versus major-feature scope, implement PRD
   supersession, allocate all internal IDs safely, require all eight sections,
   and validate the full checklist before approval.
2. `refine-user-stories`: verify source PRD approval, enforce one slice by default,
   and require explicit approval metadata for each generated story.
3. `user-story-to-gherkin`: verify story approval, use the canonical template,
   require explicit confirmation before writing, add parent/status comments, and
   run syntax/coverage validation.
4. `create-requirements`: require approved story and approved scenarios, include
   dependencies/deferred decisions/parent PRD traceability, and define draft to
   approved promotion.
5. `create-design`: distinguish revising a draft ADR from superseding an accepted
   ADR, require approved requirements, and record all related schema/ADR decisions.
6. `create-tasks`: plan verification commands without claiming their output;
   require approved packet inputs, packet-review state, and evidence placeholders.
7. Add `implement-feature` and `verify-feature` skills for the missing execution
   and completion phases.
8. Add an archival or supersession skill so completed context is moved safely.
9. Add `triage-observation` and `record-release` skills so post-release work
   follows the same interview-and-approval discipline as feature work.

**Template changes:**

1. Replace ambiguous PRD scope with an explicit enum and add `supersedes`.
2. Add typed `depends_on`, `requires`, `blockers`, approval, and packet-review
   metadata to feature artifacts.
3. Add parent/status comments and scenario IDs to Gherkin files.
4. Add scenario-level traceability columns to requirements.
5. Add completion-record and observed-command sections to tasks.
6. Rename task-item IDs to avoid collision with the task-document ID, for example
   `TASK-001-01` or a separately governed item namespace.
7. Add ADR supersession and immutability guidance to the ADR template.
8. Replace all illustrative metadata and example domain content before approval.
9. Normalize database/table type values to the documented vocabulary.
10. Add `specs/templates/supporting/observation.md` and
    `specs/templates/supporting/release.md`, plus a lightweight fix-packet
    template (`FIX-NNN`) reusing the applicable feature-packet sections.
11. Add observation and release record fields to the required frontmatter set
    validated by the specification validator.

**Deliverables:** Updated skills, templates, and skill-level completion reports.

**Exit gate:** A dry run from an approved PRD to a ready packet produces no missing
metadata, undocumented status changes, or unapproved downstream artifacts.

### Phase 4: Complete The Executable Gates

**Objective:** Make readiness and completion mechanically checkable.

**Actions:**

1. Add a repository-local specification validator, preferably as an `xtask`
   subcommand to preserve the Rust automation boundary.
2. Validate artifact IDs, collisions, required frontmatter, lifecycle values,
   parent references, dependency references, blockers, placeholders, and links.
3. Validate bidirectional database/table references.
4. Validate Gherkin parent/status metadata and one-feature-per-story rules.
5. Validate requirements-to-scenario and scenario-to-requirement coverage.
6. Validate ADR immutability and supersession references.
7. Validate packet readiness without inferring approval from file existence.
8. Validate completion records and reject claims for commands not executed.
9. Update `cargo xtask ci` to enforce all required local gates, including domain
   coverage and the approved vendored-source exclusion.
10. Add the required release, unused-dependency, and conditional Miri checks to
    repository CI.
11. Invoke `cargo xtask eval` from the retrieval-quality CI path and fail on ADR-004
    threshold regressions.
12. Add CI configuration that runs the same authoritative commands as local CI.
13. Validate observation and release records: mandatory fields, legal lifecycle
    transitions, links to specifications and regression tests, and closure
    evidence. Reject any fix that lacks a regression test or a specification
    reference.

**Deliverables:** Specification validator, updated `cargo xtask ci`, evaluation
integration, CI workflow, and state-fixture tests.

**Exit gate:** The validator rejects every intentionally invalid fixture, including
missing approval, open blockers, unresolved references, unchecked completion,
normative `TBD`, and contradictory packet status.

### Phase 5: Reconcile Existing Artifacts Conservatively

**Objective:** Bring current specifications into the new state model without
inventing historical evidence.

**Required reconciliation order:**

1. Resolve feature 010's re-ranker scope and update all affected artifacts.
2. Resolve feature 011's blocking re-ranker question and latency requirement.
3. Select the canonical update command and revise all product-facing references.
4. Select file-node identity and reconcile requirements, design, schema, and code.
5. Resolve feature 009's exact-bytes versus UTF-8-only contract.
6. Update PRD-001's stale golden-set, entity-extraction, and epic references.
7. Reconcile packet-level statuses for features 011 through 013.
8. Preserve `007` through `010` historical implementation statuses but record
   missing evidence as explicit audit debt.
9. Correct `TABLE-001`, `TABLE-005`, and `TABLE-011` so DDL and invariants agree.
10. Complete machine-readable `related`, `requires`, and `depends_on` references.
11. Replace approved-artifact `owner: TBD` values or explicitly define an approved
    ownership exception.
12. Archive packets only after completion evidence and supersession relationships
    are verified.
13. Migrate `OBS-001` through `OBS-013` from `TODO.md` into observation records
    with preserved IDs and original text, record the current baseline as the
    first release record, and keep `TODO.md` as the index of unresolved and
    deferred items.

**Deliverables:** Reconciled packet set, migration log, audit-debt register,
updated schemas, and archived/superseded artifacts where justified.

**Exit gate:** No active approved packet contains a blocking open question,
normative `TBD`, unresolved reference, contradictory public contract, or status
claim unsupported by the new evidence model.

### Phase 6: Exercise And Operate The Upgraded Workflow

**Objective:** Verify the workflow itself, not only the product code it produces.

**Fixtures to exercise:**

1. Missing PRD or missing feature packet.
2. Draft PRD, story, requirements, design, and tasks.
3. Unapproved upstream artifact.
4. Open blocker at every workflow stage.
5. Missing dependency or schema.
6. Contradictory command or identity contract.
7. Ready packet with all approvals and references.
8. Red test observed, green test observed, and failed verification.
9. Completed packet with complete evidence.
10. Superseded packet and replacement PRD.
11. Archived packet excluded from active dispatch.
12. Concurrent writer attempt rejected or serialized.
13. Observation lifecycle fixtures: new report, confirmed defect, regression on a
    released version, behavior change request, deferred technical debt,
    duplicate, and rejected report.
14. Release-loop fixtures: baseline release record, a fix packet traced through
    triage to closure, the emergency release-blocker path, and mechanical
    verification of observation-to-closure traces.

**Operational checks:**

1. Verify role write boundaries if the multi-agent control plane is enabled.
2. Verify blocked agents report `blocked` or `needs-approval`, never `completed`.
3. Verify commands not run are never reported as passing.
4. Verify current-session dependency and constitutional approvals are obtained
   from the human rather than inferred from tool permissions.
5. Verify the workflow remains usable with `opencode --pure` if OpenCode
   configuration is added.
6. Document troubleshooting and recovery for contradictory metadata and failed
   verification.

**Deliverables:** Workflow fixture suite, smoke-test results, operating guide, and
review checklist.

**Exit gate:** The workflow can resume deterministically from every supported state
without guessing, silently rewriting specifications, or dispatching unsafe work.

## Required Human Decisions Before Implementation

The following decisions should be resolved before changing workflow behavior:

1. Is `MULTI_AGENT_SDD_PLAN.md` active policy now, or is it a future design?
2. Should requirements, designs, tasks, and scenarios use the shared lifecycle
   statuses directly or use a separate packet-level status with artifact review
   states?
3. How should historical `implemented` packets with missing evidence be labeled
   without inventing evidence?
4. Is the canonical update command `mdsearch collection update` or
   `mdsearch update`?
5. Are file nodes keyed by stable `file_id` or canonical path?
6. Is file retrieval explicitly UTF-8-only, or must the CLI preserve arbitrary
   stored bytes?
7. What concrete latency target replaces the normative `TBD` in feature 011?
8. Who owns approved artifacts currently carrying `owner: TBD`?
9. Should observation records live in `specs/observations/OBS-NNN.md` or remain
   rows inside `specs/TODO.md` with added lifecycle fields?
10. What severity levels and response expectations apply to post-release
    defects?
11. Which emergency fixes may bypass the full packet, and which gates are never
    waived?
12. Where should release records live: `specs/releases/REL-NNN.md`, a changelog,
    or both?

## Definition Of Done For This Upgrade

- [ ] One authoritative workflow and state-transition model is approved.
- [ ] Approval, blocker, readiness, completion, and archival semantics are
      machine-readable and documented.
- [ ] The canonical workflow includes red-test and observed-evidence gates.
- [ ] All SDD skills enforce their upstream prerequisites and write contracts.
- [ ] Templates contain the metadata required by the skills and readiness checks.
- [ ] Accepted ADRs cannot be edited in place by the workflow.
- [ ] A specification validator rejects invalid packet states.
- [ ] `cargo xtask ci`, retrieval evaluation, and repository CI are aligned with
      the Constitution.
- [ ] Existing packets are reconciled without fabricated approval or verification
      evidence.
- [ ] Public command, identity, byte-output, and schema contracts are consistent.
- [ ] Workflow fixtures cover blocked, ready, failed, completed, superseded, and
      archived states.
- [ ] The upgraded workflow has been exercised by a human reviewer from PRD
      selection through implementation completion.
- [ ] A post-release operations phase is documented with intake, triage, fix,
      emergency, and release-closure paths.
- [ ] Observation, fix-packet, and release-record templates exist and produce
      validated artifacts.
- [ ] Existing `TODO.md` observations are migrated without ID reuse or fabricated
      evidence.
- [ ] Every released version has a release record with gate evidence, known
      observations, and rollback information.

## Audit Conclusion

The repository should retain its current specification-first direction. The
priority is not to add more process documents, but to make the existing process
authoritative, stateful, evidence-based, and mechanically verifiable.
Post-release work is held to the same standard: observations, fixes, and releases
get the same evidence-based control loop as feature work, not a separate
heavyweight process.

Until Phases 0 through 4 are complete, frontmatter status should be treated as
historical context rather than dispatch authorization or proof of completion.
