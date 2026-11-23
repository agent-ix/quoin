---
id: SR-2026112016223475846355005
title: "PLAT-1834 quoin PR 1029 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; PR 1029 (FR-200, FR-191-AC-22, risk_plan.rs, tc_1834_risk_management_plan.rs)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-200"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

# PLAT-1834 quoin PR 1029 gap-analysis

## Summary

Ticket: PLAT-1834. `quoin matrix --repo . --json` (quoin 0.36.8) on the PR head: FR-200-AC-1..11 are all `tagged` (AC-4, AC-7, AC-8, AC-9 have two binders). FR-191-AC-22, whose text this PR rewrites, is `untagged`. No stub, `todo!` or `unimplemented!` in the changed production code; no production code without an owning requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-191-AC-22 is rewritten to make the DOS-RMP exit-1 conditional on "the repository holds no plan record", but no test is tagged to it and no test drives unfiltered `generate` with and without a plan record to show the exit status follows the plan. The matrix row is `untagged`. | spec/functional/FR-191-dossier-command.md:156 |
| FND-002 | low | FR-200-AC-10 requires "dossier source exit 3 without a rendered row"; the only test calls `render()` directly and asserts `Err`. Nothing drives a blank-cell or missing-table plan through the reader and `generate` to assert exit 3 and that no `DOS-RMP.md` is written (AC-4 has such a test in tc_1656). | rust/crates/quoin-dossier/src/risk_plan.rs:508-529 |
| FND-003 | low | The policy match relies on `quoin_quire::records` reducing an `ix://owner/repo/POL-1` relationship target to `POL-1`, but no test anywhere (unit or tc_1834) authors an `ix://` target; only bare ids are exercised. An `ix://` plan reference is the repo's normal relationship form and should have one test. | rust/crates/quoin-dossier/src/risk_plan.rs:180-190 |

## Coverage

Plan completion: not assessed

| Criterion | Tests (binding) |
| --- | --- |
| FR-200-AC-5 | each_plan_section_states_the_same_missing_source (correct; count 4) |
| FR-200-AC-7 | an_authored_plan_renders_its_four_tables_and_clears_the_plan_gap; a_plan_authored_from_the_module_skeleton_renders_and_references_its_policy (correct) |
| FR-200-AC-8 | the_plan_references_its_criteria_and_does_not_copy_or_choose_them; tc_1834 skeleton test (correct) |
| FR-200-AC-9 | a_plan_that_references_no_policy_in_scope_has_a_gap_naming_the_plan; a_plan_that_names_no_policy_in_scope_is_reported_with_the_plan_as_subject (correct) |
| FR-200-AC-10 | an_unfilled_plan_fails_instead_of_rendering (correct, render level only; FND-002) |
| FR-200-AC-11 | plans_render_in_id_order_deterministically_without_a_verdict (correct, but cannot detect byte vs dossier order; code-review FND-001) |

The tc_1834 test fills the safety module's real skeleton from PR #50 and runs the real reader; it passes against spec-objects-safety dbd860e4 and needs #50 merged before #1029 for CI (`.ci-deps/spec-objects-safety` checks out main).

## Verdict

CHANGES REQUESTED (FND-001). FR-200 coverage is otherwise complete and the tests assert real behaviour.

## Examined

- FR-200-AC-1..FR-200-AC-11 (examined)
- FR-191-AC-22 (examined)
