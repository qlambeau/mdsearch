---
id: REL-NNN
title: "Release Milestone vX.Y.Z"
type: release-record
status: released
version: "vX.Y.Z"
commit: "<git-hash>"
date: YYYY-MM-DD
owner: TBD
related: []
---

# Release Record

## Release Overview

Summary of release intent, version milestone, and major capabilities delivered.

## Included Features

| Feature ID | Title | User Story | Status |
| --- | --- | --- | --- |
| 001 | Feature slug | `US-001` | implemented |

## Closed Observations & Fixes

| Observation ID | Title | Resolution |
| --- | --- | --- |
| `OBS-001` | Observation title | Promoted to EPIC-008 & implemented |

## Verification Evidence

- `cargo xtask ci`: observed green output
- `cargo xtask eval`:
  - Recall@5: 1.0 (threshold >= 0.85)
  - MRR@5: 1.0 (threshold >= 0.70)
  - NDCG@5: 1.0 (threshold >= 0.75)

## Known Open Observations

- `OBS-NNN`: Deferred to next milestone.

## Migration & Rollback

- Schema version: `N`
- Rollback procedure: TBD
