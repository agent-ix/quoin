---
id: SR-20261111201428
title: "Gap analysis of quoin PR 817 (PLAT-1404 paired-run plans in verify and report)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@d9a17f68; spec/functional/FR-132-paired-run-metrics.md AC-8..13, FR-108 reason table, spec/matrix.md, rust/crates/quoin-measurement/tests/tc_3500_paired_verify.rs, rust/crates/quoin-cli/tests/tc_3513_paired_verify_command.rs"
review_set: subset
---

# Gap analysis of quoin PR 817

## Summary

Ticket: PLAT-1404. Plan completion was not assessed. `quoin matrix` (quoin 0.30.0) on the PR
head shows FR-132-AC-1..13 all `tagged`. `quire coverage --scope .` reports FR-132 13/13.
Every TC-3500..3511 and TC-3513 test carries its row's TC id together with the criterion
that row cites.

There are no id collisions:

- TC-3500..3513 appear nowhere on origin/main or in any remote branch.
- Open PR #816 uses TC-3300..3302 and adds no FR-132 criteria.
- FR-132-AC-8..13 are new.

#816 and #817 conflict textually in `spec/matrix.md`. The FR-131 row and the FR-132 row
are adjacent, so whichever PR lands second needs a trivial rebase. Against origin/main,
GitHub reports the PR as MERGEABLE.

## Verdict

**Coverage is real. Two low gaps.** Each AC-8..13 test asserts the behaviour its AC states.
The TC-3513 end-to-end test exercises `record`, then `verify`, then `report` through the
binary: unpaired exits 1, equal exits 0 with estimate 0, and differing exits 1 with
`paired_run_differs`. The ACs cover the cookiecutter use case. A throwaway probe confirmed
it with one slice per file and 48-bit hash values.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The TC-3513 marker carries `FR-132-AC-10`, but its matrix row cites only FR-132-AC-8 and FR-132-AC-13. `quire coverage` reports `criterion-tagged-outside-row` ("expected a same-test tag for one of TC-3504, TC-3505"), so this tag backs nothing. Add AC-10 to the TC-3513 row, or drop the tag | rust/crates/quoin-cli/tests/tc_3513_paired_verify_command.rs:111, spec/matrix.md:1459 |
| FND-002 | low | FR-132 Behavior says "A plan that also states a `decision_rule` and `estimator` is judged by both". No test covers a plan with a full rule and estimator plus `paired_run`. TC-3502 covers rule-without-estimator only. The combined `decisions` list and the reason merge are unpinned | spec/functional/FR-132-paired-run-metrics.md:38, rust/crates/quoin-measurement/tests/tc_3500_paired_verify.rs:772-785 |
