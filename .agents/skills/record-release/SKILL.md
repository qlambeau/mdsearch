---
name: record-release
description: >-
  Compile a durable release record (specs/releases/REL-NNN.md) documenting included
  features, closed observations, quality gate evidence, and eval baselines.
---

# Record Release

## Purpose

This skill documents release milestones in a versioned artifact, ensuring traceability
between code commits, delivered feature packets, resolved observations, and quality metrics.

## Invocation

Examples:

- `Record release REL-001 for v0.1.0 baseline`
- `Compile release record for v0.2.0`

## Release Record Contents

Each `specs/releases/REL-NNN.md` file records:
1. **Frontmatter:** `id: REL-NNN`, `version: vX.Y.Z`, `status: released`, `commit: <hash>`, `date: YYYY-MM-DD`.
2. **Included Features:** Table of all `US-NNN` / feature packets included in the release.
3. **Closed Observations:** Table of all `OBS-NNN` resolved in this milestone.
4. **Verification Evidence:** Recorded test summary and evaluation metrics (Recall@5, MRR@5, NDCG@5).
5. **Known Open Observations:** Unresolved items deferred to future milestones.
6. **Rollback & Migration:** Database migration instructions and rollback steps.
