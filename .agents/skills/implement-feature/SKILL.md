---
name: implement-feature
description: >-
  Implement an approved feature packet test-first, following the constitutional
  RED -> GREEN -> REFACTOR -> TRACE engineering cycle.
---

# Implement Feature

## Purpose

This skill guides the implementation of a spec-ready feature packet. It strictly
enforces test-first Rust development governed by `specs/CONSTITUTION.md`.

## Invocation

Examples:

- `Implement feature specs/014-unify-query-semantics/`
- `Drive the red-to-green implementation for US-014`

## Prerequisites

1. Verify that the feature packet passes the Spec-Ready Predicate (`user-story.md`,
   `scenarios.feature`, `requirements.md`, `design.md`, `tasks.md` are all `status: approved`).
2. Read the complete `specs/CONSTITUTION.md` before writing or modifying any Rust code.
3. Read the packet's `design.md`, `tasks.md`, and related ADRs/schemas.

## Execution Rules (The Constitutional Loop)

1. **RED Tests First:**
   - Author unit, integration, or scenario tests covering the planned tasks in `tasks.md`.
   - Execute tests and observe that they fail with the expected failure output.
   - Do NOT proceed to implementation before red test output is observed.
2. **GREEN Minimal Implementation:**
   - Author the minimal production code necessary to make the red tests pass.
   - Execute tests and observe that they pass.
3. **REFACTOR & TRACE:**
   - Clean up code, remove duplication, ensure idiomatic naming and error handling.
   - Attach traceability comments (`/// Covers: REQ-NNN`) on public items and tests.
4. **Constitution Constraints:**
   - Do NOT add crate dependencies, workspace members, or layers without explicit current-session approval.
   - Do NOT use `unsafe` code.
   - Do NOT weaken tests or loosen lints.
5. **Specification Match:**
   - If implementation reveals a behavioral discrepancy, STOP and update the specification first.

## Handoff

When all ordered tasks are coded and passing locally, invoke `verify-feature` to execute
quality gates (`cargo xtask ci` and `cargo xtask eval`) and record completion evidence.
