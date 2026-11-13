---
id: SR-20261113104327
title: "Gap analysis of quoin risk discrepancies (PR 864, FR-154, FR-155)"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quoin; PR #864 (git diff origin/main...HEAD at 708e3b8b): spec/functional/FR-154, spec/functional/FR-155 against rust/crates/quoin-risk-advice/** and rust/crates/quoin-cli/{src/risk.rs,tests/tc_154_risk_discrepancies_command.rs}"
relationships:
  - {target: "ix://agent-ix/quoin/FR-154", type: references}
  - {target: "ix://agent-ix/quoin/FR-155", type: references}
---

# Gap analysis of quoin risk discrepancies (PR 864)

## Summary

Ticket: PLAT-1570. Planless gap analysis scoped to FR-154 and FR-155 (merged spec on
origin/main). Plan completion: not assessed.

The computed matrix (`quoin matrix --repo . --json`, quoin 0.30.1) shows every FR-154 and
FR-155 criterion as `untagged`. That is a tool limitation, not this PR's: the same run shows the
merged FR-140 to FR-149 criteria untagged too, and only 7 criteria tagged across the whole
repository, so the matrix does not read the Rust `#[trace(...)]` tags. The tag census was
therefore done by reading the tests. All 10 FR-154 ACs and all 13 FR-155 ACs carry at least one
`#[trace]` tag on a test that exercises them.

Examined bindings: tc_154_risk_discrepancies.rs (11 tests) and
tc_154_risk_discrepancies_command.rs (3 tests). Every production branch in discrepancies.rs has
a test except the ones named in FND-001. No stub, no coverage inflation, and no code without an
owning requirement: the `risk.rs` refactor serves FR-154's CLI, and `load` is the shared reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Only the two level reasons of the `decision-without-region` branch are tested (FR-155-AC-8). The FR-155 Behavior line "accepted or not_accepted and no region resolves" has four further paths in code, and none is tested: the policy is not loaded, the policy is ambiguous, the cell is not in the policy, and the estimate is missing or was skipped. Each emits `region_missing: null` (see SR-20261113104326 FND-002). Add one test per path once FR-155 states what `region_missing` holds for it. | rust/crates/quoin-risk-advice/src/discrepancies.rs:110-131, rust/crates/quoin-risk-advice/src/discrepancies.rs:143-192, rust/crates/quoin-risk-advice/tests/tc_154_risk_discrepancies.rs:145-174 |
| FND-002 | low | `the_json_has_the_two_keys_and_an_empty_scope_has_no_findings` is tagged FR-155-AC-12. It runs neither `--strict` nor the Markdown "the policy marks this cell as" check. AC-12 is actually covered by the CLI strict test (stdout equal with and without `--strict`) and by the Markdown test. Drop the FR-155-AC-12 tag from this test. | rust/crates/quoin-risk-advice/tests/tc_154_risk_discrepancies.rs:312-320 |
| FND-003 | low | FR-154-AC-7 asks for the "accept"-stem and no-verdict checks "in the Markdown over every fixture". The test asserts them over one fixture, which holds one `accepted-outside-acceptable-region` finding and one agreeing risk. The `not-accepted-inside`, `benefit-risk-*` and `decision-without-region` lines, all of which print a copied `accepted`/`not_accepted`, are never checked by it. Run the stem check over the Markdown of each fixture the suite builds (a shared assertion helper is enough). | rust/crates/quoin-risk-advice/tests/tc_154_risk_discrepancies.rs:333-373 |

## Verdict

Not mergeable as is: one medium finding and two low. Traceability is complete at AC level, and
the behaviour tests are real. The medium gap is in the untested no-region branches. It goes with
the FR-155 spec edit raised in the code review.
