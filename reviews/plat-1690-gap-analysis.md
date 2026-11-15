---
id: SR-20261114002435
title: "Gap analysis — quoin RMM module support"
type: SpecReview
analysis: gap-analysis
scope: "spec/, rust/crates/quoin-core/src/ops/evidence/discharge.rs, rust/crates/quoin-cli/src/discharge.rs, rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-140", type: references }
---

## Summary

Audited the repository's computed spec-to-test-to-code matrix and the PLAT-1690 change. The change implements the module eligibility behavior, but FR-140-AC-13 and AC-14 have no trace-tagged tests; repository-wide gaps are also present outside this PR.

## Verdict

**FAIL** — two criteria directly exercised by the new code path remain untagged, and the repository matrix reports additional untagged criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The test for a publishable module and retained `pack: false` behavior does not carry trace tags for FR-140-AC-13 or AC-14. | FR-140-AC-13, FR-140-AC-14; rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs::tc_1488_005_no_licensed_field_reaches_any_output_and_the_search_can_fail |
| FND-002 | high | The repository-wide matrix reports 565 additional untagged criteria outside FR-140-AC-13 and AC-14. | quoin matrix output |
| FND-003 | high | Two repository criteria are backed only by ignored tests, so no runnable test discharges them. | quoin matrix output |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (Quoin 0.34.0; Quire engine 0.60.0); static and evidence axes read.
- Plan completion: not assessed.
- Criteria: tagged 1,165; untagged 567 (2 in FR-140 AC-13/14 and 565 elsewhere); tagged-by-ignored-test 2; method-without-symbol 133; retired 37.
- Evidence: no run evidence for 1,904 criteria.
- `quire coverage --scope . --json`: 263 untracked symbols and 1,387 diagnostics, repository-wide.
- Untraced behavior/stubs in changed source: none observed; the changed discharge behavior is owned by FR-140.
- Semantic review: skipped; not requested.

The baseline `origin/main` also reports extensive repository-wide matrix gaps. It has 587 untagged criteria, while this branch has 567; the two FR-140 criteria described above remain untagged on this branch.
