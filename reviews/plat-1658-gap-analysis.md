---
id: SR-1625
title: "Gap analysis — PLAT-1658 records-of-processing generator (Quoin PR #962)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-207-dossier-records-of-processing-content.md, spec/test/tc-617-records-of-processing-dossier.md, rust/crates/quoin-dossier/src/records_of_processing.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-207", type: references}
---

## Summary

The computed Quoin matrix on review head `8589b1ab` reported FR-207-AC-1..8 as tagged. Gap-analysis FND-001 noted that TC-617's fixture supplied `region` as a scalar, while the Quire flow-report contract exposes it as a list; the test could not detect the region-list comparison defect. On current PR head `2c9be0f2`, TC-617 supplies multiple labels and checks mixed and all-matching cases, so the missing coverage is repaired. Plan completion: not assessed.

## Verdict

FND-001 is fixed on current PR head. FR-207-AC-1 now has a test binding that exercises list-valued region facts and the exact label comparison. The review did not assess unrelated repository-wide matrix gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-617 uses a scalar region fixture and does not exercise the flow-report region list contract | rust/crates/quoin-dossier/src/records_of_processing.rs:879-953 |

## Coverage

- Matrix: `quoin matrix --repo /home/peter/dev/quoin/.worktrees/plat1658-review-head --json`; FR-207-AC-1..8 were tagged on review head.
- Reviewed FR-207 AC-1 against its test binding and source implementation.
- Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Current TC-617 fixture uses `region: ["us", "eu"]` and `region: ["eu", "eea"]`; assertions cover one mixed-match sink and one all-matching sink, exercising list-valued flow facts. |
