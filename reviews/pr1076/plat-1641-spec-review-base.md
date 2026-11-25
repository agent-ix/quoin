---
id: SR-967175026302197768900474181941935084779
title: "Base spec review — validation report"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-217-validation-report.md, spec/functional/FR-220-external-tool-configuration-index.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-217", type: references }
  - { target: "ix://agent-ix/quoin/FR-220", type: references }
---

## Summary

Reviewed the changed FR-217 anomaly requirements and the FR-220 external-tool obligations implemented by this PR. The IDs and links validate, but the report contract has an ambiguous composite criterion and four criteria without a runnable binder.

## Verdict

**FAIL** — live report behavior remains untested and FR-217-AC-8 is not implemented in this repository slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-217-AC-5 says to render “exactly the two” carried anomalies and also to list a closed `wont-fix` anomaly, leaving unclear whether the expected result is two or three rows when one is present. | FR-217-AC-5 |
| FND-002 | high | FR-217-AC-8 has no test binder and the quire-cli/quire-rs record behavior is deferred outside this PR. | FR-217-AC-8 |
| FND-003 | high | FR-220-AC-2 has no test binder for matching run evidence to index entries. | FR-220-AC-2 |
| FND-004 | high | FR-220-AC-3 has no test binder for unavailable-tool handling. | FR-220-AC-3 |
| FND-005 | high | FR-220-AC-4 has no test binder for missing/duplicate index entries and identity mismatches. | FR-220-AC-4 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified on PR head 79eaa3e8. |
| FND-002 | deferred | FR-217-AC-8 is Quire-side work tracked separately by PLAT-1642, which remains open; it is outside this PLAT-1641 replacement PR. |
| FND-003 | fixed | Verified on PR head 79eaa3e8. |
| FND-004 | fixed | Verified on PR head 79eaa3e8. |
| FND-005 | fixed | Verified on PR head 79eaa3e8. |

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
- fnd: FND-002
  outcome: deferred
  reason: FR-217-AC-8 is Quire-side work tracked separately by PLAT-1642, which remains
    open; it is outside this PLAT-1641 replacement PR.
- fnd: FND-003
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-2")]'
- fnd: FND-004
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-3")]'
- fnd: FND-005
  outcome: fixed
  after_excerpt: '#[trace("FR-220-AC-4")]'
```

+++
