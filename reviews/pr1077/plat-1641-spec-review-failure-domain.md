---
id: SR-967175026302197768900474181941935084784
title: "Failure-domain review — validation report"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-217-validation-report.md, spec/functional/FR-236-selftest-run-record.md, spec/functional/FR-162-anomaly-report.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-217", type: references }
---

## Summary

Reviewed anomaly-set identity and the empty/absent cases across report, run-record, and selection requirements. Empty and absent outcomes are specified, but the report's source set is not bound to the exact producer version when reports are generated later from retained runs.

## Verdict

**FAIL** — a report can show anomaly information from a different binary version than the one that produced the retained run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-217 requires anomalies carried by the run-producing binary, but FR-236 retains only the tool-version string and FR-217 does not require the producer's selected anomaly set to be retained or resolvable. A later report invocation therefore has no specified way to reproduce that version's anomaly set. | FR-217, FR-236 |
## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified on PR head 79eaa3e8. |

+++ [reviewer dispositions]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "- The self-test recorder SHALL add an anomaly-set object to each\
    \ run record with a provided flag,\n  the producer version, and each embedded\
    \ anomaly document's path and raw content.\n- The self-test recorder SHALL set\
    \ the anomaly-set producer version to the exact toolVersion recorded for that\
    \ run."
```

+++
