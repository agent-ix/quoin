---
id: SR-20261117175274
title: "Gap analysis — PLAT-1792 FR-185 acceptance criteria to tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-185-in-toto-test-result-export.md, rust/crates/quoin-evidence/tests/tc_test_result_export.rs, rust/crates/quoin-evidence/tests/tc_test_result_provenance.rs, rust/crates/quoin-cli/tests/tc_test_result_export.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-185", type: references }
---

## Summary

This audit is scoped to the PR diff. It maps the FR-185 criteria that PR #1014 touches to the tests that are tagged with them. Ticket: PLAT-1792. Plan completion: not assessed.

## Verdict

**CHANGES REQUESTED.** AC-10 is fully backed. AC-11 and AC-12 are tagged on tests that cover only part of each criterion. One new test carries the wrong criterion tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-185-AC-11's second sentence, "a source-workflow test compares the signed Test Matrix to the complete Quire matrix at `sourceRevision` and detects an omitted active obligation", has no test. tc_1792_011 and tc_1792_102 are tagged AC-11 but cover only the suites.md digest half. | rust/crates/quoin-evidence/tests/tc_test_result_provenance.rs:59; rust/crates/quoin-cli/tests/tc_test_result_export.rs:368 |
| FND-002 | medium | The AC-12 clauses "invalid-signature, untrusted ... provenance also refuses export" are unbacked and unimplemented: the tests tagged AC-12 accept a fabricated `{"sig":"AAAA"}`. The tag overclaims coverage until AC-12 is amended (see the integrity review, FND-001). | rust/crates/quoin-cli/tests/tc_test_result_export.rs:325,368; rust/crates/quoin-evidence/tests/tc_test_result_provenance.rs:43 |
| FND-003 | low | tc_1792_103 tests that output is preserved on refusal or write failure but is tagged FR-185-AC-3 (subject validation). It belongs to AC-6. Its write-failure branch also accepts exit 0 when directory permissions are bypassed, so under root it asserts nothing. | rust/crates/quoin-cli/tests/tc_test_result_export.rs:450-482 |

## Recommended fixes

- FND-001: add the source-workflow matrix-completeness test under FR-186, or move that sentence from FR-185-AC-11 to FR-186-AC-7 and drop the AC-11 overclaim.
- FND-002: amend AC-12 so that signature and trust checks are assigned to the Engineering Assurance verifier, then retag.
- FND-003: retag tc_1792_103 as FR-185-AC-6.

## Scope

- FR-185-AC-3 (examined): subject validation, now including the revision annotation; covered by tc_185_subject_is_preserved_and_bad_subjects_are_refused.
- FR-185-AC-6 (examined): missing or malformed provenance leaves no output; covered by tc_1792_102 and tc_1792_103, both mis-tagged.
- FR-185-AC-8 (examined): a duplicate failure cannot pass; covered by tc_1792_003.
- FR-185-AC-10 (examined): an omitted obligation fails and unrelated bindings do not change the population; covered by tc_1792_001, 002 and 100.
- FR-185-AC-11 (examined): partially covered.
- FR-185-AC-12 (examined): partially covered.
