---
id: SR-20261117175240
title: "Code review — PLAT-1630 clause approval enforcement"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-assurance/src/discharge/{build.rs,mod.rs,pack_rmm.rs,rmm.rs,store.rs}; rust/crates/quoin-assurance/tests/discharge.rs; rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs; rust/crates/quoin-core/src/{capabilities.rs,runtime.rs,ops/evidence/discharge.rs,ops/modules/support.rs}; rust/crates/quoin-quire/src/clauses.rs; FR-208"
review_set: subset
---

## Summary

Reviewed the FR-208 runtime and pack-discharge changes in Quoin PR #981, including Rust implementation, boundaries, and tagged tests. The runtime applies approval policies from loaded module metadata and preserves supplied references, but it refuses blank references before the required open-work classification can run.

## Verdict

**CONDITIONAL** — one medium finding remains in the FR-208-AC-1 input path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Blank `approvalRef` values are rejected instead of leaving required clauses open | rust/crates/quoin-assurance/src/discharge/parse.rs:38 |

## Review Scope

- Rust API, exact clause-set metadata loading, discharge classification, RMM mapping, and pack report integration were examined across every changed production file listed in `scope`.
- New tests in `rust/crates/quoin-assurance/tests/discharge.rs`, `rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs`, and `rust/crates/quoin-quire/src/clauses.rs` were examined for behavior, trace tags, and assertion strength.
- FR-208 and its acceptance criteria and constraints were examined; FR-046 direct-reference behavior was read as context.
- No applicable AssuranceProfile was present under `spec/`.

## Method and Gates

- Rust review: applied repository Rust conventions and the API, implementation, wire-boundary, and testing checks.
- `cargo fmt --all --check`: passed.
- Focused FR-208 Rust tests: pending at artifact creation.
- `cargo deny` was not run; its known RustSec findings are assigned to the owner lane and are outside this review's remit.
- No changes to CI workflow files were present in the PR diff.

## Verdict Detail

The changes resolve approval roles against installed clause-set metadata, gate only the direct-fact population, leave empty-role policies on FR-046 behavior, reject duplicate clause-set declarations, and retain caller-supplied approval references without adding authentication claims. The tagged test matrix covers FR-208-AC-1 through AC-4; AC-5 is specified for inspection and was inspected directly. The missing blank-reference path is recorded above.

## Findings Detail

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | code-bug | FR-208-AC-1 requires an empty or whitespace-only approval reference to remain open work. `parse_fact` sends every present `approvalRef` through `string_value`, which rejects blank text before `classify_binding_clause` can classify the fact as open. The command therefore refuses the fact instead of producing the required open-work report. |

## Scope Records

- FR-208-AC-1: A binding clause with non-empty `approvalRoles` remains open when its direct fact has no `approvalRef`, or has an empty or whitespace-only reference. (examined)
- FR-208-AC-2: A binding clause with non-empty `approvalRoles` that require a named human approver is direct only when its direct fact has a non-blank `approvalRef` identifying that person; the report retains the reference without changing the direct classification. (examined)
- FR-208-AC-3: A binding clause with empty `approvalRoles` continues to accept a direct fact without `approvalRef` as specified by FR-046. (examined)
- FR-208-AC-4: When multiple installed modules declare the same exact clause-set identity, discharge fails without selecting either module's approval policy. (examined)
- FR-208-AC-5: The report preserves a supplied approval reference and does not claim to authenticate the named approver, role, or authorization represented by it. (examined)
- FR-208-CON-1: A clause set's approval requirement applies only to facts for the exact clause-set identity and clause ID declared by that set. (examined)
- FR-208-CON-2: When the Quoin runtime processes approval metadata, it SHALL NOT calculate a risk score, apply a risk threshold, or make an automated risk-acceptability decision. (examined)
- FR-208: Quoin SHALL apply approval requirements declared by the installed clause set when it discharges facts against a validated binding report. (examined)
- FR-046 direct facts: A direct evidence fact MAY carry an approvalRef that identifies a human approval associated with the evidence. (context only)
