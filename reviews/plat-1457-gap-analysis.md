---
id: SR-20261112014601
title: "Gap analysis of quoin PR 822 (PLAT-1457 report decides paired-run plans)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@f847c866; spec/functional/FR-132-paired-run-metrics.md AC-12, spec/functional/FR-108-measurement-verdict-checker.md AC-8, spec/matrix.md TC-3600..3606, rust/crates/quoin-measurement/tests/tc_3600_report_order.rs, rust/crates/quoin-cli/tests/tc_3513_paired_verify_command.rs"
review_set: subset
---

# Gap analysis of quoin PR 822

## Summary

Ticket: PLAT-1457. Plan completion was not assessed. `quoin matrix` (quoin 0.30.0) on the PR
head lists FR-132-AC-12 and FR-108-AC-8 as `tagged`, with no `criterion-tagged-outside-row`
diagnostic. Every TC-3600..3606 test carries `FR-132-AC-12` together with its row's TC id.
TC-3600..3606 do not exist on origin/main (c694a2d6) or in open PR #819. The NFR-widening
lane uses 3700..3799.

## Verdict

**One medium coverage gap. It is the gap that hid code-review FND-001.** The unit tests
cover these cases:

- three collections with a git order
- a contradicting timestamp
- no order, shallow and tie, each with three collections
- two collections under `equal` with no order
- `not_worse` in both directions

The CLI tests cover a git repository and no git state (no `.git`, empty `PATH`). The
three-collection report row is asserted on the real CLI.

The `agree` helper checks one direction only. It requires the report to be inconclusive when
verify adds `order_unattested`. The two-collection shallow case is never constructed, and it
is exactly the case where the two diverge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers a shallow clone with two collections under `equal`/`within_delta`. TC-3602 builds the shallow order with three collections only, where both sides are unattested anyway. TC-3603 covers two collections only with `IntakeOrder::none()`, where verify adds no plan-level finding. The case where verify adds the plan-level shallow `order_unattested` and the report decides is untested, and that is where the two disagree (code-review FND-001, reproduced). Add it to TC-3602 (or as a TC-3603 variant) asserting both sides give the same answer | rust/crates/quoin-measurement/tests/tc_3600_report_order.rs:225-238, rust/crates/quoin-measurement/tests/tc_3600_report_order.rs:240-255 |
