---
id: SR-084
title: "Integrity review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: integrity
scope: "US-018, FR-062, TM-001 TC-1249..TC-1260"
review_set: all
---

# Integrity review of issue 152 graph-analysis requirements

## Summary

US-018 traces through FR-062 to StR-004, and FR-062 carries explicit verification for every
criterion. The three views share one input boundary and report model without conflicting with the
evidence auditor's sole ownership of verdicts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved during review: the CLI's export, acceptance-premise, and verdict sources were implicit. Required `--export`, `--premises`, and `--audit` paths now close those hidden inputs without granting `--repo` producer or auditor behavior. | FR-062; TC-1257 |

## Traceability conclusion

`StR-004 -> US-018 -> FR-062 -> TC-1249..TC-1260` is complete. FR-062's external dependency is
not treated as an ambient tool assumption: quire-rs FR-067/FR-068 own a versioned, schema-validated
offline payload, the caller supplies the payload, accepted premises, and source-bound audit
explicitly, and mismatches fail closed. The report never performs collection or
reinterprets an auditor verdict, so its responsibility does not conflict with FR-030 or FR-032.
