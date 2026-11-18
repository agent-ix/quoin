---
id: SR-20261117175256
title: "Gap analysis — Quoin VER-112 implementation (PR #994)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; repository spec/, rust/; PR #994 changed coverage parser and test"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-204", type: references }
---

## Summary

Planless, repository-driven matrix verification and reverse-gap review for Quoin PR #994. The change closes the FR-204-AC-9 tag gap; the repository still has 617 untagged criteria, down from 618 on main before this change. Plan completion was not assessed.

## Verdict

**FAIL** — the repository matrix still has untagged criteria outside this PR's scope; the specific VER-112 criterion is now tagged.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | FR-001-AC-1 remains untagged. It is representative of 617 untagged criteria in this repository matrix; main had 618 before PR #994 added the FR-204-AC-9 test. | spec/functional/FR-001-parse-command-line.md:50 |

## Coverage

- Reconciliation: `quoin matrix --repo <candidate> --json` (Quoin 0.36.7; Quire 0.52.0, engine 0.62.2); no grep fallback and no run evidence.
- Plan completion: not assessed
- Criteria: tagged 1401 · untagged 617 · tagged-by-ignored-test 5 · method-without-symbol 146 · retired 37 (total 2206)
- Evidence: no run evidence for all 2206 criteria
- Reverse gap and stub scan of the changed implementation: 0 untraced behaviors, 0 source stubs, 0 test stubs. The new diagnostic/source-locus behavior is owned by FR-204-AC-9.
- Semantic review: skipped (not requested).
- Baseline comparison: clean main at 1adc7d47 had 1400 tagged and 618 untagged; PR #994 adds a binder for FR-204-AC-9 and does not change the other untagged criteria.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | FR-001-AC-1 is explicitly listed as `None (no native test is tagged to this criterion; PLAT-1045)` in the unchanged verification cell, and it was already untagged on clean main (618 untagged criteria). PR #994 changes only formal-rules malformed-document diagnostics and adds the FR-204-AC-9 binder; it neither introduces nor claims to resolve the parser criterion. This repository-level pre-existing gap is out of scope for this PR and is not an actionable defect in its change. |
