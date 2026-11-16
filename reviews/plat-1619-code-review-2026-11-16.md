---
id: SR-20261116004342
title: "Code and Rust review of PLAT-1619 unresolved anomalies dossier"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1619 PR #947 patch against peter/plat-1618-soup-document: rust/crates/quoin-dossier/Cargo.toml, rust/crates/quoin-dossier/src/anomalies.rs, rust/crates/quoin-dossier/src/gap.rs, rust/crates/quoin-dossier/src/lib.rs, rust/crates/quoin-dossier/tests/tc_1619_anomalies.rs, rust/Cargo.lock"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-198"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Reviewed only PR #947's patch against `peter/plat-1618-soup-document`; the base is PR #944, and its changes are excluded from this review. The generator consumes the FR-162 anomaly report, preserves source grouping and carried-row order, renders authored rationale and assessments, maps missing assessment and gate failures to dossier gaps, and reports source failures through the command frame. The code and test diff show no defect against FR-198 and FR-191. No independent build or test run was performed in this review; the coder reported focused check, Clippy and integration checks passing.

## Verdict

**PASS** — no findings in the PR's own patch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope Examined

| Unit | Role | Evidence |
| --- | --- | --- |
| FR-198-AC-1 | examined | `tc_1619_anomalies.rs` checks source-selected open, resolved, wont-fix, and fixed rows. |
| FR-198-AC-2 | examined | Integration test checks unassessed gap behavior. |
| FR-198-AC-3 | examined | Integration test checks source gate failures and absence of suggested rationale. |
| FR-198-AC-4 | examined | Source test checks byte stability when git-derived change values differ; renderer uses authored rationale only. |
| FR-198-AC-5 | examined | Integration test checks hazard and criterion groups and the detailed anomaly list. |
| FR-198-AC-6 | examined | Integration test checks unreadable source failure and no output. |
| FR-198-AC-7 | examined | Integration test checks deterministic rendering. |
| FR-198-AC-8 | examined | Integration test checks ungrouped rows and exact reasons. |
| FR-198 | context_only | Source-selection, authored-rationale, and git-derived-field rules used to interpret the ACs. |
| FR-191 | context_only | Common command-frame source-failure behavior and generator-registration contract. |

## Review Evidence

- Reviewed the exact merge-base diff: `rust/Cargo.lock`, dossier crate dependency metadata, anomaly generator, gap enum/order, generator registry, and `tc_1619_anomalies.rs`.
- Manually checked output escaping through the existing `line` and `cell` helpers and source-failure handling for unreadable report and missing gate input.
- The PR adds no workflow or build-gate changes.
- User direction and FR-198 both exclude rendering git-derived `rationaleChange` and `fixingChange`; the implementation omits them.
- No test or build commands were independently rerun by this reviewer.
