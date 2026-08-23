---
name: verify-feature
description: >-
  Execute all repository quality gates, record verification output, and update
  feature packet completion evidence in tasks.md.
---

# Verify Feature

## Purpose

This skill executes the Definition of Done checks for a completed feature packet.
It runs all automated gates, verifies retrieval quality, and records concrete,
observable evidence into `tasks.md` before transitioning the packet to `implemented`.

## Invocation

Examples:

- `Verify implementation of specs/014-unify-query-semantics/`
- `Run verification gates and record evidence for US-014`

## Verification Gates

1. **Specification Validator:**
   ```bash
   cargo xtask validate-specs
   ```
2. **Repository CI Suite:**
   ```bash
   cargo xtask ci
   ```
   (Runs formatting, clippy `-D warnings`, workspace tests `--all-features`, docs `-D warnings`, cargo deny, and llvm-cov line coverage >= 85%).
3. **Retrieval Quality Evaluation (when applicable):**
   ```bash
   cargo xtask eval --verify-only
   ```
   (Enforces ADR-004 thresholds: Recall@5 >= 0.85, MRR@5 >= 0.70, NDCG@5 >= 0.75).

## Evidence Recording

1. Record observed command output, date, and commit hash in `tasks.md` under `## Test And Verification Plan` and `## Definition Of Done`.
2. Check all completed task boxes `[x]`.
3. Transition `tasks.md` frontmatter to `status: implemented`.
4. Report the observed verification summary.
