---
id: SR-20261116075229
title: "Gap analysis — PLAT-1767 Quoin Quire matrix bridge"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md, rust/crates/quoin-core/src/ops/quire/mod.rs, rust/crates/quoin-core/src/ops/quire/tests.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-115", type: references}
---

## Summary

Ticket: PLAT-1767. Planless; plan completion: not assessed. The focused binding for FR-115-AC-7 is backed by two non-ignored tests, including the changed adapter projection test. A full repository gap audit was not available: installed Quoin 0.35.0 refused the current architecture module's `record_keys` field, while a direct Quire matrix run with the manually assembled module set exited 3 on unresolved module relationship and role diagnostics.

## Verdict

**CONDITIONAL** — the changed criterion's tag and test binding are verified, but repository-wide matrix, reverse-gap, and stub checks could not be completed with the installed tool/module combination.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found | - |

## Coverage

| Unit | Role | Result |
| --- | --- | --- |
| `FR-115-AC-7` | examined | `quire matrix` output binds the changed trace-tagged test and the existing matrix test; both bind the criterion. |
| `rust/crates/quoin-core/src/ops/quire/mod.rs` | examined | The bridge maps Quire's current `UnmarkedTraceId` claim reason. |
| `rust/crates/quoin-core/src/ops/quire/tests.rs` | examined | The engine-shaped test fixture uses the current claim variant and remains tagged to FR-115-AC-7. |

Full repository matrix, reverse-gap and stub checks: unavailable because the installed Quoin 0.35.0/engine 0.60.0 could not load the architecture module schema containing `record_keys`. A direct Quire 0.51.0/engine 0.62.2 run with explicit module roots emitted unresolved `UnknownEdgeType` / `UnknownRole` diagnostics and exited 3, so its repository-wide result is not treated as a completed audit. Optional semantic review was skipped. Plan completion: not assessed.
