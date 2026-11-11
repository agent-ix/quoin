---
id: SR-16394
title: "integrity review of PLAT-1021 graded-corpus collection producer"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-108-measurement-verdict-checker.md (FR-108-AC-15..AC-21, Graded-corpus section, Dependencies), spec/matrix.md (FR-108 row) -- PR #779 diff only"
review_set: subset
---

## Summary

Ticket: PLAT-1021. Ids, internal consistency and cross-references in the
diff. AC ids continue after AC-14 with AC-13 withdrawn; TC-2013..TC-2020 are
unused on `main`; `quoin_jev::ConstantPredictorItem` is the real re-export of
`constant_predictor::Item`; the refusal list in AC-18 matches what
`item_observations` refuses, and AC-18 lists an empty `metric` itself (which
`item_observations` does not catch, since the suffixed item metric is never
empty). The matrix row is consistent with the ACs. One low wording finding.

## Scope examined

- FR-108-AC-15 .. AC-21 (examined)
- FR-108 Dependencies bullet (examined)
- spec/matrix.md FR-108 row (examined)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low | AC-17 says the item rows are "exactly the rows item_observations builds for the same items", but item_observations keeps input order while AC-19 orders rows by item_id; it should say the rows item_observations builds for the items in item_id order | FR-108-AC-17; FR-108-AC-19 |

## Dispositions

| FND | Outcome | Reason |
| --- | ------- | ------ |
| FND-001 | fixed | Round 1: AC-17 now reads "for the same items and metric, in item_id order (FR-108-AC-19)". |
