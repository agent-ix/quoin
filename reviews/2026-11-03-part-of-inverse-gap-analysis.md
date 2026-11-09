---
id: SR-20261103118115
title: "Gap analysis — Quoin relationship mapping"
type: SpecReview
analysis: gap-analysis
scope: "spec/, rust/crates/quoin-core/src/ops/quire/, rust/crates/quoin-semantic/tests/, tests/fixtures/semantic-module/mapping/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-104"
    type: references
---

## Summary

Repository-driven audit of Quoin's relationship mapping criteria, computed matrix, tests, and changed source. Plan completion was not assessed. The added relationship regression now binds FR-104-AC-1, while the repository-wide matrix retains existing untagged criteria.

## Verdict

**FAIL** — 607 criteria remain untagged in the repository-wide computed matrix; the changed relationship criterion is tagged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | 607 acceptance criteria have no test bound by the computed matrix; FR-104-AC-1 is covered by the new fixture regression | computed Quoin matrix; spec/ |

## Coverage

- Reconciliation: quoin matrix --repo /private/tmp/quoin-part-of --json (Quoin 0.29.3, Quire 0.38.1 / engine 0.51.0); no run evidence was read.
- Criteria: 492 tagged, 607 untagged, 0 tagged-by-ignored-test, 81 method-without-symbol; 1180 total.
- Evidence: no run evidence.
- Plan completion: not assessed
- Reverse gap and stubs: rechecked changed source and tests. The coverage status projection remains under its existing coverage contract; the new relationship test binds FR-104-AC-1. No new unowned CLI behavior or stub was introduced.
- Semantic review: skipped.
