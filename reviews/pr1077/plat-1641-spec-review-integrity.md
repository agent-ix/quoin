---
id: SR-967175026302197768900474181941935084781
title: "Integrity review — validation report"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-217-validation-report.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-217", type: references }
---

## Summary

Reviewed FR-217's new anomaly behavior, acceptance criteria, dependency entries, and validation links for completeness, consistency, and atomicity. The criterion IDs are sequential and the new FR-162 edge is consistent, but AC-5 conflates the exact open-row count with the additional closed `wont-fix` behavior.

## Verdict

**CONDITIONAL** — the expected row count should be stated without the current ambiguity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-5's “exactly the two carried anomalies” can be read as excluding the separately required closed `wont-fix` row; say “exactly the two open anomalies, plus any carried closed `wont-fix` anomaly.” | FR-217-AC-5 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified on PR head 79eaa3e8. |

+++ [reviewer dispositions]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '| FR-217-AC-5 | Given a version-matched retained anomaly set with
    an open anomaly assessed against a `high_process_risk` FR, an open anomaly assessed
    against a `not_high_process_risk` FR, a closed `fixed` anomaly, and a closed `wont-fix`
    anomaly, the `Known anomalies` section renders exactly the two open anomalies
    and the closed `wont-fix` anomaly, highlights the high-risk row first, and omits
    the closed `fixed` anomaly. | Test |'
```

+++
