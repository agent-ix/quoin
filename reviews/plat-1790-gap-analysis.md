---
id: SR-20261123063610
title: "Gap analysis of Quoin PR 1021 (PLAT-1790 owner-qualification swap coverage)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@8a4fc24f2987ea291d89d05f48a0b005cc0dd353; spec/functional/FR-116-owner-qualified-quire-evidence.md, spec/functional/FR-184-analysis-act-producer-definition.md, rust/crates/quoin-quire/tests/analysis_counting.rs, rust/crates/quoin-cli/tests/tc_ver80_differential_report_command.rs, rust/crates/quoin-evidence/src/qualify/**"
review_set: subset
---

## Summary

Ticket: PLAT-1790. This audit is scoped to the PR diff and the criteria it traces. It is planless. Plan completion: not assessed.

Every id the PR traces resolves in `spec/` at the reviewed head: FR-116-AC-6/7/8/10/11/12/14/15/18, FR-184-AC-17/18 and FR-153-AC-8. A repo-wide `quoin matrix` run was not made; this was a diff-scoped audit. Semantic intent-to-test review was done inline for the traced criteria; see the code-review artifact SR-20261123063609. No stubs and no production code were added.

## Verdict

**CONDITIONAL.** Two medium gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-116's status text still says "normative owner contract; implementation remains planned", calls the bridge "planned", and says "All criteria below describe planned implementation obligations. This contract claims no executable acceptance evidence". This contradicts the shipped `quoin_evidence::qualify` bridge and the tests this PR traces to FR-116. The ticket asked for this status fix. The PR's "no draft text" claim is literally true but leaves the stale status in place. | spec/functional/FR-116-owner-qualified-quire-evidence.md:28-32, 43-44, 206, 231-232 |
| FND-002 | medium | The FR-116 result categories `selection-mismatch`, `invalid-target` and `ambiguous-target` (FR-116 Outputs table, AC-8 and AC-13) have no counterpart in the qualifier's `Cause` enum, so no code produces them. They are traced but unimplemented. The PR does not record this as a confirmed defect with a regression test. | rust/crates/quoin-evidence/src/qualify/mod.rs:40-59; spec/functional/FR-116-owner-qualified-quire-evidence.md:113-115 |

## Coverage

- Traced criteria in the diff: 12. All resolve. The tag-to-intent mismatches are recorded in SR-20261123063609 (FR-116-AC-8 selection clause, FR-116-AC-10 reused-name clause, FR-184-AC-17 positive case).
- Reverse gap: none. The diff adds only tests.
- Stubs and coverage inflation: no stubs. Vacuous assertions are recorded in SR-20261123063609 (FND-002, FND-006).
- Plan completion: not assessed
