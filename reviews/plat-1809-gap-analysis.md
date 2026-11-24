---
id: SR-136
title: "Gap analysis — risk decisions PR #1048"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; repository spec/, rust/; changed FR-142, FR-145, FR-191, FR-199, FR-213, FR-214, NFR-031 and changed Rust crates"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin", type: references }
---

## Summary

Ran the engine-backed matrix over the repository and inspected changed Rust behavior for reverse gaps and stubs. All 18 criteria newly introduced by this PR are tagged; the repository has substantial pre-existing untagged criteria and stale trace symbols, and the SOUP producer/join interaction is recorded separately in SR-135.

## Verdict

**FAIL** — the whole-repository matrix reports 647 untagged criteria and 5 criteria backed only by ignored tests. Six existing bindings also disappear relative to `origin/main`; the new criteria themselves are covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository matrix reports 647 untagged criteria; 6 criteria that were tagged on `origin/main` are untagged at this head. | FR-161-AC-15, FR-161-AC-16, FR-161-AC-17, FR-165-AC-20, FR-193-AC-9, FR-194-AC-9 |
| FND-002 | high | Five repository criteria are backed only by ignored tests, including the existing FR-142-AC-19 binding. | FR-142-AC-19 |
| FND-003 | medium | The coverage index reports 291 untracked trace symbols; these have no live criterion binding. | quire coverage --scope . |

## Coverage

- Reconciliation: quoin matrix (quoin 0.36.8, quire 0.52.0 / engine 0.62.2).
- Plan completion: not assessed.
- Criteria: tagged 1569 · untagged 647 · tagged-by-ignored-test 5 · method-without-symbol 153 · retired 36.
- Evidence: no run evidence for all 2410 criteria.
- New criteria: all 18 added by PR #1048 are tagged.
- Relative to `origin/main`, 6 previously tagged criteria become untagged: FR-161-AC-15/16/17, FR-165-AC-20, FR-193-AC-9 and FR-194-AC-9. Their source files are unchanged in this diff. This requires follow-up attribution.
- Reverse-gap/stub scan: no TODO, FIXME, `todo!` or `unimplemented!` in the changed production files. The SOUP multi-component producer/join mismatch is covered in SR-135.
- Semantic review: skipped (not opted in).
- Coverage extraction also reports 291 untracked symbols, 1680 diagnostics, including five cfg-dependent symbol-id collisions; no failure in the changed criteria's tags was found.


## Dispositions

| FND | outcome | reason | after_excerpt |
| --- | --- | --- | --- |
| FND-001 | still-open | The current engine matrix still reports 647 untagged criteria, including the same six previously tagged criteria that lost bindings versus origin/main; attribution/fixes are not present in this PR head. | - |
| FND-002 | deferred | The five ignored-test-only bindings, including FR-142-AC-19, are not changed by this PR; this is an existing repository-wide coverage issue outside the scoped implementation. | - |
| FND-003 | deferred | The matrix still reports 291 untracked trace symbols, with no change to that repository-wide inventory in this PR; cleanup is outside the scoped implementation. | - |
