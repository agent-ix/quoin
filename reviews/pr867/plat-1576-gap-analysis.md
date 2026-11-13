---
id: SR-20261113125830
title: "Gap analysis of quoin risk-summary (PR 867, FR-169, FR-170)"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quoin; PR #867 (git diff origin/main...HEAD at 6fd570a5): spec/functional/FR-169, spec/functional/FR-170 against rust/crates/quoin-risk-advice/** and rust/crates/quoin-cli/{src/main.rs,src/risk.rs,tests/tc_169_risk_summary_command.rs}"
relationships:
  - {target: "ix://agent-ix/quoin/FR-169", type: references}
  - {target: "ix://agent-ix/quoin/FR-170", type: references}
---

# Gap analysis of quoin risk-summary (PR 867)

## Summary

Ticket: PLAT-1576. A planless gap analysis scoped to FR-169 and FR-170, as merged on
origin/main. Plan completion: not assessed. Semantic review: not run as a separate fan-out.
The test-to-AC binding was read by hand for every tagged test.

The computed matrix (`quire matrix --scope . --format json`, quoin 0.30.1) shows all 26
FR-169 criteria and all 16 FR-170 criteria as `tagged`. The tags sit on
tc_169_risk_summary.rs (18 tests) and tc_169_risk_summary_command.rs (4 tests). There is no
stub, no coverage inflation, and no code without an owning requirement. summary.rs, the
`decided_by` reader field, the `risk-summary` Clap command and the exit-code branch all serve
FR-169/170. The MP-250/251 case is NFR-003's.

Most tags are backed by real assertions. The gaps are where a tagged test does not assert the
whole criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-170-AC-10 has two halves. The residual-risk findings must equal `risk discrepancies`, and the control findings must equal the control findings of `risk advise` for the same controls. The tagged test compares only the first half. It filters `control-without-requirement`, `control-without-verification` and `post-control-without-control` out of the row, and never reads `evidence_gaps` from `advise`. Because summary.rs restates those control rules (SR-20261113125829 FND-002), the missing comparison is the only guard against drift. Add it: for each control in the row, its tokens equal the tokens of the matching `ControlEvidence` in `advise`. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:389-480 |
| FND-002 | medium | FR-169-AC-19 asks for a snapshot corpus with "one chain with each gap of FR-170". The snapshot holds 8 of the 13 tokens. Missing are `post-control-estimate-without-decision`, `residual-risk-undecided`, `accepted-outside-acceptable-region`, `benefit-risk-not-accepted` and `not-accepted-inside-acceptable-region`. Also, FR-170-AC-1 (finding keys over a fixture holding every token) is tagged on `a_complete_chain_is_one_row_with_exact_keys_and_no_findings`, whose row has `findings: []`, so that test asserts no finding keys. Extend the snapshot corpus to every token, which needs further residual risks and an unreferenced post-control estimate. Then move the FR-170-AC-1 tag, and the FR-170-AC-13 tag, to the snapshot test or to a key check over it. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:135-184, rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:798-850, rust/crates/quoin-risk-advice/tests/snapshots/risk_summary.md |
| FND-003 | low | FR-169-AC-20 lists "a scope that exists but is unreadable". No test covers it: the library and CLI tests cover a missing scope, a missing module and a bad benefit-risk header only. Also, the library test `a_missing_scope_and_an_undeclared_type_use_the_advise_codes` tests only the missing scope. The undeclared type is covered by the CLI test only. Add a `chmod 000` scope case beside the missing scope, and rename the library test or add the undeclared-type case to it. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:738-746, rust/crates/quoin-cli/tests/tc_169_risk_summary_command.rs:138-208 |
| FND-004 | low | FR-169-AC-13 asks that a `null` region from a marker, from `policy-not-found` and from `policy-ambiguous` each render `policy_region=null` in Markdown, "over every fixture". The Markdown test renders only the `policy-not-found` case (EST-C has no residual risk). The marker case appears only in the snapshot. No Markdown test renders `policy-ambiguous`. The "accept"-stem check runs over one fixture with no `not_accepted` value or `benefit-risk-*`/`not-accepted-*` token. Render the AC-25 two-policy fixture and the snapshot corpus as Markdown and run the same region and stem assertions over them. | rust/crates/quoin-risk-advice/tests/tc_169_risk_summary.rs:630-690 |

## Verdict

CONDITIONAL, not mergeable as is: two medium findings and two low. Traceability is complete at
AC level, and the behaviour tests are real. The medium gaps are half of FR-170-AC-10 and the
snapshot corpus required by FR-169-AC-19. Both are test-only fixes.

## Coverage

- FR-169: 26/26 criteria tagged. FR-170: 16/16 criteria tagged.
- Partially asserted criteria: FR-170-AC-10, FR-169-AC-19, FR-170-AC-1, FR-169-AC-20 and
  FR-169-AC-13 (findings above).
- Plan completion: not assessed
- Semantic review: skipped as a separate pass. The bindings were read by hand.
